import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

type LauncherItem = {
  id: string;
  title: string;
  subtitle?: string | null;
  icon?: string | null;
  kind: "app" | "file" | "folder" | "text" | "clipboard" | "script";
};

function App() {
  const [query, setQuery] = useState("");
  const [items, setItems] = useState<LauncherItem[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [isLaunching, setIsLaunching] = useState(false);

  useEffect(() => {
    let cancelled = false;

    async function search() {
      try {
        const results = await invoke<LauncherItem[]>("search_apps", { query });

        if (!cancelled) {
          setItems(results);
          setSelectedIndex(0);
          setError(null);
        }
      } catch (caught) {
        if (!cancelled) {
          setError(String(caught));
        }
      }
    }

    search();

    return () => {
      cancelled = true;
    };
  }, [query]);

  async function launchSelected(item = items[selectedIndex]) {
    if (!item || isLaunching) {
      return;
    }

    setIsLaunching(true);
    setError(null);

    try {
      await invoke("launch_app", { appId: item.id });
    } catch (caught) {
      setError(String(caught));
    } finally {
      setIsLaunching(false);
    }
  }

  function handleKeyDown(event: React.KeyboardEvent<HTMLInputElement>) {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      setSelectedIndex((index) => Math.min(index + 1, items.length - 1));
    }

    if (event.key === "ArrowUp") {
      event.preventDefault();
      setSelectedIndex((index) => Math.max(index - 1, 0));
    }

    if (event.key === "Enter") {
      event.preventDefault();
      launchSelected();
    }
  }

  return (
    <main className="launcher-shell">
      <section className="launcher-card">
        <div className="launcher-header">
          <span className="eyebrow">KLauncher</span>
          <span className="count">{items.length} apps</span>
        </div>

        <input
          autoFocus
          className="search-input"
          onChange={(event) => setQuery(event.currentTarget.value)}
          onKeyDown={handleKeyDown}
          placeholder="Search apps..."
          spellCheck={false}
          value={query}
        />

        {error ? <p className="error">{error}</p> : null}

        <div className="results" role="listbox">
          {items.map((item, index) => (
            <button
              className={`result ${index === selectedIndex ? "selected" : ""}`}
              key={item.id}
              onClick={() => launchSelected(item)}
              onMouseEnter={() => setSelectedIndex(index)}
              role="option"
              type="button"
            >
              <span className="icon-box">{item.icon ? item.icon.slice(0, 2) : "A"}</span>
              <span className="result-copy">
                <span className="result-title">{item.title}</span>
                <span className="result-subtitle">{item.subtitle ?? item.id}</span>
              </span>
            </button>
          ))}
        </div>
      </section>
    </main>
  );
}

export default App;
