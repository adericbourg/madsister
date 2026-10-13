import { X } from "lucide-react";

export type TabLabel = { id: number; title: string; isDirty: boolean };

type Props = { tabs: TabLabel[]; activeId: number; panelId: (id: number) => string; onSelect: (id: number) => void; onClose: (id: number) => void };

export const tabId = (id: number) => `song-tab-${id}`;

/** One tab per open song (F-UI-7). The close buttons sit next to the tabs, not inside them. */
export const TabBar = ({ tabs, activeId, panelId, onSelect, onClose }: Props) => (
  <div
    role="tablist"
    aria-label="Songs"
    className="tabbar"
    onKeyDown={(e) => {
      const step = ({ ArrowRight: 1, ArrowLeft: -1 } as Record<string, number>)[e.key];
      if (step === undefined) return;
      const i = tabs.findIndex((t) => t.id === activeId);
      const next = tabs[(i + step + tabs.length) % tabs.length].id;
      onSelect(next);
      document.getElementById(tabId(next))?.focus();
    }}
  >
    {tabs.map(({ id, title, isDirty }) => (
      <div key={id} role="none" className={id === activeId ? "tab is-active" : "tab"}>
        <button type="button" role="tab" id={tabId(id)} aria-selected={id === activeId} aria-controls={panelId(id)} tabIndex={id === activeId ? 0 : -1} onClick={() => onSelect(id)}>
          {isDirty && <span title="Unsaved changes">•</span>}
          {title}
        </button>
        <button type="button" className="tab-close" title={`Close ${title}`} onClick={() => onClose(id)}>
          <X size={14} />
          <span className="visually-hidden">Close {title}</span>
        </button>
      </div>
    ))}
  </div>
);
