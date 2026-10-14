import { useState, type Ref } from "react";
import type { DisplayStyle } from "../model/display";
import type { MinorConvention } from "../model/nashville";
import { CHART_FONTS, isThreshold, type ChartFont, type Settings } from "./Toolbar";

type Props = { ref: Ref<HTMLDialogElement>; settings: Settings; onSettingsChange: (settings: Settings) => void };

/** The user settings (F-UI-3), applied on Save; any other way out discards the edits. Opened with `showModal()` on its ref. */
export function Preferences({ ref, settings: saved, onSettingsChange }: Props) {
  const [edits, setEdits] = useState<Partial<Settings>>({});
  const settings = { ...saved, ...edits };
  const set = (edit: Partial<Settings>) => setEdits({ ...edits, ...edit });
  const close = (e: { currentTarget: Element }) => {
    setEdits({});
    e.currentTarget.closest("dialog")!.close();
  };
  return (
    <dialog ref={ref} className="preferences" aria-labelledby="preferences-title" onClick={(e) => e.target === e.currentTarget && close(e)} onCancel={() => setEdits({})}>
      <div>
        <h2 id="preferences-title">Preferences</h2>
        <label>
          Chord style{" "}
          <select value={settings.style} onChange={(e) => set({ style: e.target.value as DisplayStyle })}>
            <option value="fr">French (C7M)</option>
            <option value="intl">International (Cmaj7)</option>
            <option value="latin">Latin (Sol7M)</option>
          </select>
        </label>
        <label>
          Font{" "}
          <select value={settings.font} onChange={(e) => set({ font: e.target.value as ChartFont })}>
            {CHART_FONTS.map(([value, label]) => (
              <option key={value} value={value}>
                {label}
              </option>
            ))}
          </select>
        </label>
        <label>
          Nashville numbers in minor keys{" "}
          <select value={settings.minorConvention} onChange={(e) => set({ minorConvention: e.target.value as MinorConvention })}>
            <option value="relative">From the relative major (Am: 6m)</option>
            <option value="tonic">From the minor tonic (Am: 1m)</option>
          </select>
        </label>
        <label>
          Bars per row{" "}
          <select value={settings.barsPerRow} onChange={(e) => set({ barsPerRow: Number(e.target.value) as Settings["barsPerRow"] })}>
            <option>2</option>
            <option>4</option>
            <option>8</option>
          </select>
        </label>
        <label>
          Review chords below confidence{" "}
          <input
            type="number"
            min={0}
            max={1}
            step={0.05}
            value={settings.lowConfidenceThreshold}
            onChange={(e) => {
              const n = Number(e.target.value);
              if (e.target.value !== "" && isThreshold(n)) set({ lowConfidenceThreshold: n });
            }}
          />
        </label>
        <label>
          <input type="checkbox" checked={settings.compactPrint} onChange={(e) => set({ compactPrint: e.target.checked })} /> Compact print (2 columns)
        </label>
        <label>
          <input type="checkbox" checked={settings.reopenTabs} onChange={(e) => set({ reopenTabs: e.target.checked })} /> Reopen previous tabs on launch
        </label>
        <div className="actions">
          <button type="button" autoFocus onClick={close}>
            Cancel
          </button>
          <button
            type="button"
            className="primary"
            onClick={(e) => {
              onSettingsChange(settings);
              close(e);
            }}
          >
            Save
          </button>
        </div>
      </div>
    </dialog>
  );
}
