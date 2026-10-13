import { FilePlus, FolderOpen, Printer, Save, Settings } from "lucide-react";

type Props = {
  hasSong: boolean;
  isEditing: boolean;
  onNew: () => void;
  onOpen: () => void;
  onSave: () => void;
  onPrint: () => void;
  onToggleMode: () => void;
  onPreferences: () => void;
};

/** The most frequent actions (F-UI-6); the rest is in the native menu. */
export const Ribbon = ({ hasSong, isEditing, onNew, onOpen, onSave, onPrint, onToggleMode, onPreferences }: Props) => (
  <nav aria-label="File" className="ribbon">
    <button type="button" onClick={onNew}>
      <FilePlus size={16} /> New
    </button>
    <button type="button" onClick={onOpen}>
      <FolderOpen size={16} /> Open…
    </button>
    <button type="button" className="primary" disabled={!hasSong} onClick={onSave}>
      <Save size={16} /> Save
    </button>
    <button type="button" disabled={!hasSong} onClick={onPrint}>
      <Printer size={16} /> Print…
    </button>
    <span className="ribbon-end">
      {hasSong && (
        <label className="mode-switch">
          View
          <input type="checkbox" role="switch" aria-label="Edit mode" checked={isEditing} onChange={onToggleMode} />
          Edit
        </label>
      )}
      <button type="button" onClick={onPreferences}>
        <Settings size={16} /> Preferences
      </button>
    </span>
  </nav>
);
