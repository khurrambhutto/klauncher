"""Target only this launcher's X11 window for synthetic key events and image capture."""
import ctypes as C
from ctypes.util import find_library


class KeyEvent(C.Structure):
    _fields_ = [("type", C.c_int), ("serial", C.c_ulong), ("send_event", C.c_int),
                ("display", C.c_void_p), ("window", C.c_ulong), ("root", C.c_ulong),
                ("subwindow", C.c_ulong), ("time", C.c_ulong), ("x", C.c_int),
                ("y", C.c_int), ("x_root", C.c_int), ("y_root", C.c_int),
                ("state", C.c_uint), ("keycode", C.c_uint), ("same_screen", C.c_int)]


class Event(C.Union):
    _fields_ = [("key", KeyEvent), ("pad", C.c_long * 24)]


class XImage(C.Structure):
    _fields_ = [("width", C.c_int), ("height", C.c_int), ("xoffset", C.c_int),
                ("format", C.c_int), ("data", C.c_void_p), ("byte_order", C.c_int),
                ("bitmap_unit", C.c_int), ("bitmap_bit_order", C.c_int),
                ("bitmap_pad", C.c_int), ("depth", C.c_int), ("bytes_per_line", C.c_int),
                ("bits_per_pixel", C.c_int), ("red_mask", C.c_ulong),
                ("green_mask", C.c_ulong), ("blue_mask", C.c_ulong)]


class OwnWindow:
    def __init__(self, pid):
        self.pid = pid
        self.x = C.CDLL(find_library("X11"))
        signatures = {
            "XOpenDisplay": ([C.c_char_p], C.c_void_p),
            "XDefaultRootWindow": ([C.c_void_p], C.c_ulong),
            "XInternAtom": ([C.c_void_p, C.c_char_p, C.c_int], C.c_ulong),
            "XGetWindowProperty": ([C.c_void_p, C.c_ulong, C.c_ulong, C.c_long, C.c_long,
                C.c_int, C.c_ulong, C.POINTER(C.c_ulong), C.POINTER(C.c_int),
                C.POINTER(C.c_ulong), C.POINTER(C.c_ulong), C.POINTER(C.c_void_p)], C.c_int),
            "XFree": ([C.c_void_p], C.c_int),
            "XStringToKeysym": ([C.c_char_p], C.c_ulong),
            "XKeysymToKeycode": ([C.c_void_p, C.c_ulong], C.c_uint),
            "XSendEvent": ([C.c_void_p, C.c_ulong, C.c_int, C.c_long, C.POINTER(Event)], C.c_int),
            "XSync": ([C.c_void_p, C.c_int], C.c_int),
            "XGetGeometry": ([C.c_void_p, C.c_ulong, C.POINTER(C.c_ulong),
                C.POINTER(C.c_int), C.POINTER(C.c_int), C.POINTER(C.c_uint), C.POINTER(C.c_uint),
                C.POINTER(C.c_uint), C.POINTER(C.c_uint)], C.c_int),
            "XGetImage": ([C.c_void_p, C.c_ulong, C.c_int, C.c_int, C.c_uint, C.c_uint,
                C.c_ulong, C.c_int], C.POINTER(XImage)),
            "XDestroyImage": ([C.POINTER(XImage)], C.c_int),
            "XCloseDisplay": ([C.c_void_p], C.c_int),
        }
        for name, (arguments, result) in signatures.items():
            function = getattr(self.x, name)
            function.argtypes = arguments
            function.restype = result
        self.display = self.x.XOpenDisplay(None)
        assert self.display, "No X11 display"
        self.root = self.x.XDefaultRootWindow(self.display)
        self.window = next((window for window in self.property(self.root, "_NET_CLIENT_LIST")
                            if self.property(window, "_NET_WM_PID") == [pid]), None)
        if self.window is None:
            self.close()
            raise AssertionError("Launcher X11 window is not mapped yet")

    def property(self, window, name):
        atom = self.x.XInternAtom(self.display, name.encode(), False)
        actual_type, actual_format, length, remaining = C.c_ulong(), C.c_int(), C.c_ulong(), C.c_ulong()
        data = C.c_void_p()
        status = self.x.XGetWindowProperty(self.display, window, atom, 0, 4096, False, 0,
            C.byref(actual_type), C.byref(actual_format), C.byref(length), C.byref(remaining), C.byref(data))
        try:
            if status != 0 or actual_format.value != 32 or not data.value:
                return []
            values = C.cast(data, C.POINTER(C.c_ulong))
            return [values[i] for i in range(length.value)]
        finally:
            if data.value:
                self.x.XFree(data)

    def key(self, name, control=False):
        assert self.property(self.window, "_NET_WM_PID") == [self.pid]
        code = self.x.XKeysymToKeycode(self.display, self.x.XStringToKeysym(name.encode()))
        assert code, name
        for event_type, mask in [(2, 1), (3, 2)]:
            event = Event()
            event.key = KeyEvent(type=event_type, send_event=True, display=self.display,
                window=self.window, root=self.root, same_screen=True,
                state=4 if control else 0, keycode=code)
            assert self.x.XSendEvent(self.display, self.window, False, mask, C.byref(event))
        self.x.XSync(self.display, False)

    def capture(self, path):
        from PIL import Image
        assert self.property(self.window, "_NET_WM_PID") == [self.pid]
        root, x, y = C.c_ulong(), C.c_int(), C.c_int()
        width, height, border, depth = C.c_uint(), C.c_uint(), C.c_uint(), C.c_uint()
        assert self.x.XGetGeometry(self.display, self.window, C.byref(root), C.byref(x),
            C.byref(y), C.byref(width), C.byref(height), C.byref(border), C.byref(depth))
        image = self.x.XGetImage(self.display, self.window, 0, 0, width.value, height.value, C.c_ulong(-1), 2)
        assert image
        try:
            frame = image.contents
            assert frame.bits_per_pixel == 32 and frame.byte_order == 0
            assert (frame.red_mask, frame.green_mask, frame.blue_mask) == (0xff0000, 0xff00, 0xff)
            pixels = C.string_at(frame.data, frame.bytes_per_line * frame.height)
            Image.frombytes("RGB", (frame.width, frame.height), pixels, "raw", "BGRX", frame.bytes_per_line).save(path)
        finally:
            self.x.XDestroyImage(image)

    def close(self):
        if self.display:
            self.x.XCloseDisplay(self.display)
            self.display = None
