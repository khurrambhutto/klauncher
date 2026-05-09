import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

type LauncherItem = {
  id: string;
  title: string;
  subtitle?: string | null;
  icon?: string | null;
  kind: "app" | "file" | "folder" | "text" | "clipboard" | "script";
};

function iconUrl(path: string | undefined | null): string | null {
  if (!path) return null;
  return `icon://localhost${path}`;
}

function iconFallback(title: string): string {
  const trimmed = title.trim();
  if (trimmed.length === 0) return "A";
  const first = trimmed.codePointAt(0) ?? 65;
  return String.fromCodePoint(first);
}

// Helper function to highlight matching characters
function HighlightText({ text, query }: { text: string; query: string }) {
  if (!query) return <>{text}</>;
  
  const normalizedText = text.toLowerCase();
  const normalizedQuery = query.toLowerCase();
  const chars = normalizedQuery.split('');
  
  const result: React.ReactNode[] = [];
  let textIndex = 0;
  let queryIndex = 0;
  
  while (textIndex < text.length) {
    const currentChar = normalizedText[textIndex];
    const queryChar = normalizedQuery[queryIndex];
    
    if (queryIndex < chars.length && currentChar === queryChar) {
      // This character matches the query - underline it
      result.push(
        <span key={textIndex} className="highlight-match">
          {text[textIndex]}
        </span>
      );
      queryIndex++;
    } else {
      // Regular character
      result.push(text[textIndex]);
    }
    textIndex++;
  }
  
  return <>{result}</>;
}

// SVG Icons
function SettingsIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" />
      <circle cx="12" cy="12" r="3" />
    </svg>
  );
}

function AppIcon({ icon, title }: { icon?: string | null; title: string }) {
  const [failed, setFailed] = useState(false);
  const url = useMemo(() => iconUrl(icon), [icon]);

  useEffect(() => {
    setFailed(false);
  }, [icon]);

  if (url && !failed) {
    return (
      <img
        className="icon-img"
        src={url}
        alt=""
        onError={() => setFailed(true)}
      />
    );
  }

  return <span className="icon-fallback">{iconFallback(title)}</span>;
}

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

    if (event.key === "Escape") {
      event.preventDefault();
      invoke("hide_window").catch(() => {});
    }
  }

  const selectedItem = items.length > 0 ? items[selectedIndex] : null;

  return (
    <main className="launcher-shell">
      <div className="launcher-container">
        {/* Preview Card - Top Section with Icon and Name */}
        <div className="preview-card">
          <div className="preview-box">
            {selectedItem ? (
              <AppIcon icon={selectedItem.icon} title={selectedItem.title} />
            ) : (
              <div className="preview-placeholder" />
            )}
          </div>
          <span className="preview-app-name">
            {selectedItem ? (
              <HighlightText text={selectedItem.title} query={query} />
            ) : (
              "Select an app"
            )}
          </span>

          {error && (
            <div className="error-banner">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
                <circle cx="12" cy="12" r="10" />
                <line x1="12" x2="12" y1="8" y2="12" />
                <line x1="12" x2="12.01" y1="16" y2="16" />
              </svg>
              {error}
            </div>
          )}
        </div>

        {/* Hidden input for keyboard search */}
        <input
          autoFocus
          className="hidden-search-input"
          onChange={(event) => setQuery(event.currentTarget.value)}
          onKeyDown={handleKeyDown}
          placeholder=""
          spellCheck={false}
          value={query}
        />

        {/* Results Card - Below with gap */}
        <div className="results-card">
          {/* Results Header with Search Query */}
          <div className="results-header">
            <span className="results-count">
              {items.length} {items.length === 1 ? "Result" : "Results"}
              {query && (
                <span className="query-text"> "{query}"</span>
              )}
            </span>
            <div className="results-actions">
              <button className="action-btn" title="Settings">
                <SettingsIcon />
              </button>
            </div>
          </div>

          {/* Results List */}
          <div className="results-list" role="listbox">
            {items.length === 0 ? (
              <div className="empty-state">
                <svg className="empty-state-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
                  <circle cx="11" cy="11" r="8" />
                  <path d="m21 21-4.3-4.3" />
                </svg>
                <span className="empty-state-text">
                  {query ? "No apps found" : "Start typing to search"}
                </span>
              </div>
            ) : (
              items.map((item, index) => (
                <button
                  className={`result-item ${index === selectedIndex ? "selected" : ""}`}
                  key={item.id}
                  onClick={() => launchSelected(item)}
                  onMouseEnter={() => setSelectedIndex(index)}
                  role="option"
                  type="button"
                >
                  <div className="result-icon">
                    <AppIcon icon={item.icon} title={item.title} />
                  </div>
                  <div className="result-content">
                    <span className="result-title">
                      <HighlightText text={item.title} query={query} />
                    </span>
                    <span className="result-subtitle">{item.subtitle ?? item.id}</span>
                  </div>
                </button>
              ))
            )}
          </div>
        </div>
      </div>
    </main>
  );
}

export default App;
