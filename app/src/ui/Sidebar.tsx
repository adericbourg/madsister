import { ArrowUpDown, Columns2, Gauge, Music, PanelRightClose, PanelRightOpen, Rows3, SeparatorVertical, type LucideIcon } from "lucide-react";
import { useEffect, useState, type ReactNode, type Ref } from "react";

/** The per-song panels (F-UI-4), in sidebar order; the editing ones only show in edit mode. */
const PANELS = {
  song: ["Song", Music],
  transpose: ["Transpose", ArrowUpDown],
  section: ["Section", Rows3],
  bar: ["Bar", Columns2],
  "bar-lines": ["Bar lines", SeparatorVertical],
  tempo: ["Tempo", Gauge],
} satisfies Record<string, [string, LucideIcon]>;

type PanelId = keyof typeof PANELS;

const VIEW_PANELS: PanelId[] = ["song", "transpose"];
const EDIT_PANELS = Object.keys(PANELS) as PanelId[];

export const Panel = ({ id, disabled, children }: { id: PanelId; disabled?: boolean; children: ReactNode }) => {
  const [title, Icon] = PANELS[id];
  return (
    <fieldset id={`panel-${id}`} className="toolbar" disabled={disabled}>
      <legend>
        <Icon size={16} /> {title}
      </legend>
      {children}
    </fieldset>
  );
};

type Props = {
  isCollapsed: boolean;
  isEditing: boolean;
  onCollapsedChange: (isCollapsed: boolean) => void;
  /** Where the Editor portals its panels. */
  panelsRef: Ref<HTMLDivElement>;
  children: ReactNode;
};

export const Sidebar = ({ isCollapsed, isEditing, onCollapsedChange, panelsRef, children }: Props) => {
  const [target, setTarget] = useState<PanelId | null>(null);
  // Scrolls once the panels are shown again.
  useEffect(() => {
    if (target === null) return;
    document.getElementById(`panel-${target}`)?.scrollIntoView({ block: "start" });
    setTarget(null);
  }, [target]);
  const ToggleIcon = isCollapsed ? PanelRightOpen : PanelRightClose;
  return (
    <aside id="parameters" aria-label="Parameters" className={isCollapsed ? "is-collapsed" : undefined}>
      <div className="rail">
        <button type="button" title={isCollapsed ? "Show parameters" : "Hide parameters"} aria-expanded={!isCollapsed} aria-controls="panels" onClick={() => onCollapsedChange(!isCollapsed)}>
          <ToggleIcon size={18} />
          <span className="visually-hidden">Parameters</span>
        </button>
        {isCollapsed &&
          (isEditing ? EDIT_PANELS : VIEW_PANELS).map((id) => {
            const [title, Icon] = PANELS[id];
            return (
              <button
                key={id}
                type="button"
                title={title}
                onClick={() => {
                  onCollapsedChange(false);
                  setTarget(id);
                }}
              >
                <Icon size={18} />
                <span className="visually-hidden">{title}</span>
              </button>
            );
          })}
      </div>
      <div id="panels" className="panels" hidden={isCollapsed} ref={panelsRef}>
        {children}
      </div>
    </aside>
  );
};
