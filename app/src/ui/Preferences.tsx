import { useRef } from "react";
import type { DisplayStyle } from "../model/display";
import type { MinorConvention } from "../model/nashville";
import { CHART_FONTS, isThreshold, type ChartFont, type Settings } from "./Toolbar";

type Props = { settings: Settings; onSettingsChange: (settings: Settings) => void };

/** The user settings (F-UI-3), applied as they change. */
export function Preferences({ settings, onSettingsChange }: Props) {
  const dialog = useRef<HTMLDialogElement>(null);
  const close = () => dialog.current?.close();
  return (
    <>
      <button type="button" onClick={() => dialog.current?.showModal()}>
        Preferences
      </button>
      <dialog ref={dialog} className="preferences" aria-labelledby="preferences-title" onClick={(e) => e.target === e.currentTarget && close()}>
        <div>
          <h2 id="preferences-title">Preferences</h2>
          <label>
            Chord style{" "}
            <select value={settings.style} onChange={(e) => onSettingsChange({ ...settings, style: e.target.value as DisplayStyle })}>
              <option value="fr">French (C7M)</option>
              <option value="intl">International (Cmaj7)</option>
              <option value="latin">Latin (Sol7M)</option>
            </select>
          </label>
          <label>
            Font{" "}
            <select value={settings.font} onChange={(e) => onSettingsChange({ ...settings, font: e.target.value as ChartFont })}>
              {CHART_FONTS.map(([value, label]) => (
                <option key={value} value={value}>
                  {label}
                </option>
              ))}
            </select>
          </label>
          <label>
            Nashville numbers in minor keys{" "}
            <select value={settings.minorConvention} onChange={(e) => onSettingsChange({ ...settings, minorConvention: e.target.value as MinorConvention })}>
              <option value="relative">From the relative major (Am: 6m)</option>
              <option value="tonic">From the minor tonic (Am: 1m)</option>
            </select>
          </label>
          <label>
            Bars per row{" "}
            <select value={settings.barsPerRow} onChange={(e) => onSettingsChange({ ...settings, barsPerRow: Number(e.target.value) as Settings["barsPerRow"] })}>
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
                if (e.target.value !== "" && isThreshold(n)) onSettingsChange({ ...settings, lowConfidenceThreshold: n });
              }}
            />
          </label>
          <label>
            <input type="checkbox" checked={settings.compactPrint} onChange={(e) => onSettingsChange({ ...settings, compactPrint: e.target.checked })} /> Compact print (2 columns)
          </label>
          <button type="button" autoFocus onClick={close}>
            Close
          </button>
        </div>
      </dialog>
    </>
  );
}
