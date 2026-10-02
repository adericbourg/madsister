import { useEffect, useId, useState } from "react";
import { setTransposition, transposition } from "../model/history";
import type { DisplayStyle } from "../model/display";
import { setKey, setNotation } from "../model/commands";
import type { MinorConvention } from "../model/nashville";
import type { Notation, Song } from "../model/song";
import { transposeSong, type Spelling } from "../model/transpose";
import { hasConfidence } from "./keymap";
import type { useHistory } from "./useHistory";

/** User preferences (spec F-ED-1, F-DS-1), stored in `<appConfigDir>/settings.json`, not in the song. */
export type Settings = { style: DisplayStyle; barsPerRow: 2 | 4 | 8; lowConfidenceThreshold: number; compactPrint: boolean; font: ChartFont; minorConvention: MinorConvention };

const CHART_FONTS = [
  ["kalam", "Kalam"],
  ["patrick-hand", "Patrick Hand"],
  ["petaluma-script", "Petaluma Script"],
  ["petaluma-text", "Petaluma Text"],
  ["serif", "Serif"],
] as const;

export type ChartFont = (typeof CHART_FONTS)[number][0];

const isThreshold = (v: unknown): v is number => typeof v === "number" && v >= 0 && v <= 1;

/** Anything missing or invalid falls back to the default. */
export const parseSettings = (json: unknown): Settings => {
  const raw = (typeof json === "object" && json !== null ? json : {}) as Record<string, unknown>;
  return {
    style: raw.style === "intl" || raw.style === "latin" ? raw.style : "fr",
    barsPerRow: raw.barsPerRow === 2 || raw.barsPerRow === 8 ? raw.barsPerRow : 4,
    lowConfidenceThreshold: isThreshold(raw.lowConfidenceThreshold) ? raw.lowConfidenceThreshold : 0.5,
    compactPrint: raw.compactPrint === true,
    font: CHART_FONTS.find(([value]) => value === raw.font)?.[0] ?? "petaluma-script",
    minorConvention: raw.minorConvention === "tonic" ? "tonic" : "relative",
  };
};

type FieldProps = {
  label: string;
  value: string;
  type?: "text" | "number";
  /** Returns an error message, or null when the text is valid. */
  check?: (text: string) => string | null;
  onCommit: (text: string) => void;
};

/** A text input committed on Enter or blur (one history entry per change, not per keystroke). */
export const Field = ({ label, value, type = "text", check = () => null, onCommit }: FieldProps) => {
  const [text, setText] = useState(value);
  const [error, setError] = useState<string | null>(null);
  const errorId = useId();
  useEffect(() => setText(value), [value]);
  const commit = () => {
    const trimmed = text.trim();
    const message = check(trimmed);
    setError(message);
    if (message === null && trimmed !== value) onCommit(trimmed);
  };
  return (
    <span className="field">
      <label>
        {label}{" "}
        <input
          type={type}
          value={text}
          aria-invalid={error !== null}
          aria-describedby={error === null ? undefined : errorId}
          onChange={(e) => setText(e.target.value)}
          onBlur={commit}
          onKeyDown={(e) => e.key === "Enter" && commit()}
        />
      </label>
      {error !== null && (
        <span id={errorId} role="alert" className="field-error">
          {error}
        </span>
      )}
    </span>
  );
};

type Props = { history: ReturnType<typeof useHistory>; settings: Settings; onSettingsChange: (settings: Settings) => void };

export const Toolbar = ({ history, settings, onSettingsChange }: Props) => {
  const [spelling, setSpelling] = useState<Spelling>("sharp");
  const { meta } = history.song;
  const setMeta = (patch: Partial<Song["meta"]>) => history.apply((s) => ({ ...s, meta: { ...s.meta, ...patch } }));
  // A key-less Nashville song keeps its chords relative to C: transposing would break that.
  const isTransposeLocked = meta.notation === "nashville" && meta.key === undefined;
  const offset = transposition(history.song); // net semitones since the song was opened, so Reset can undo them
  const transpose = (semitones: number) =>
    history.apply((s) => setTransposition(transposeSong(s, semitones, spelling), transposition(s) + semitones));
  return (
    <div className="toolbar">
      <fieldset>
        <legend>Display</legend>
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
          <select
            value={settings.barsPerRow}
            onChange={(e) => onSettingsChange({ ...settings, barsPerRow: Number(e.target.value) as Settings["barsPerRow"] })}
          >
            <option>2</option>
            <option>4</option>
            <option>8</option>
          </select>
        </label>
        {hasConfidence(history.song) && (
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
        )}
        <label>
          <input type="checkbox" checked={settings.compactPrint} onChange={(e) => onSettingsChange({ ...settings, compactPrint: e.target.checked })} /> Compact print (2 columns)
        </label>
      </fieldset>
      <fieldset disabled={isTransposeLocked}>
        <legend>Transpose</legend>
        <label>
          Spelling{" "}
          <select value={spelling} onChange={(e) => setSpelling(e.target.value as Spelling)}>
            <option value="sharp">Sharps (♯)</option>
            <option value="flat">Flats (♭)</option>
          </select>
        </label>
        <button type="button" onClick={() => transpose(-1)}>
          −1 semitone
        </button>
        <button type="button" onClick={() => transpose(1)}>
          +1 semitone
        </button>
        <button type="button" disabled={offset % 12 === 0} onClick={() => transpose(-offset)}>
          Reset
        </button>
      </fieldset>
      <fieldset>
        <legend>Song</legend>
        <Field label="Title" value={meta.title} check={(t) => (t === "" ? "a song needs a title" : null)} onCommit={(title) => setMeta({ title })} />
        <Field label="Artist" value={meta.artist ?? ""} onCommit={(t) => setMeta({ artist: t || undefined })} />
        <Field
          label="Key"
          value={meta.key ?? ""}
          check={(t) => (t === "" || /^[A-G][#b]?m?$/.test(t) ? null : "a key is a note, optionally followed by m (e.g. F#m)")}
          onCommit={(t) => history.apply((s) => setKey(s, t || undefined, settings.minorConvention))}
        />
        <label>
          Notation{" "}
          <select
            value={meta.notation ?? "chords"}
            disabled={meta.key === undefined}
            aria-describedby={meta.key === undefined ? "notation-hint" : undefined}
            onChange={(e) => history.apply((s) => setNotation(s, e.target.value as Notation))}
          >
            <option value="chords">Chords</option>
            <option value="nashville">Nashville numbers</option>
          </select>
        </label>
        {meta.key === undefined && (
          <span id="notation-hint" className="field-hint">
            {meta.notation === "nashville" ? "Set a key to see the chords" : "Set a key to switch to Nashville numbers"}
          </span>
        )}
        <Field
          label="Tempo (BPM)"
          type="number"
          value={meta.tempoBpm?.toString() ?? ""}
          check={(t) => (t === "" || Number(t) > 0 ? null : "the tempo must be a positive number")}
          onCommit={(t) => setMeta({ tempoBpm: t === "" ? undefined : Number(t) })}
        />
      </fieldset>
    </div>
  );
};
