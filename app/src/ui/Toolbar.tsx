import { useEffect, useId, useState } from "react";
import { setTransposition, transposition } from "../model/history";
import type { DisplayStyle } from "../model/display";
import { setKey, setNotation } from "../model/commands";
import type { MinorConvention } from "../model/nashville";
import type { Notation, Song } from "../model/song";
import { transposeSong, type Spelling } from "../model/transpose";
import { Panel } from "./Sidebar";
import type { useHistory } from "./useHistory";

/** User preferences (spec F-ED-1, F-DS-1), stored in `<appConfigDir>/settings.json`, not in the song. */
export type Settings = { style: DisplayStyle; barsPerRow: 2 | 4 | 8; lowConfidenceThreshold: number; compactPrint: boolean; font: ChartFont; minorConvention: MinorConvention; isSidebarCollapsed: boolean };

export const CHART_FONTS = [
  ["kalam", "Kalam"],
  ["patrick-hand", "Patrick Hand"],
  ["petaluma-script", "Petaluma Script"],
  ["petaluma-text", "Petaluma Text"],
  ["serif", "Serif"],
] as const;

export type ChartFont = (typeof CHART_FONTS)[number][0];

export const isThreshold = (v: unknown): v is number => typeof v === "number" && v >= 0 && v <= 1;

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
    isSidebarCollapsed: raw.isSidebarCollapsed === true,
  };
};

/** The `madsister print` option of each `Settings` key (print.rs `parse_args`). */
const OPTIONS: Record<string, string> = { style: "--style", font: "--font", barsPerRow: "--bars-per-row", compactPrint: "--compact" };

/** The saved settings with the command line's (`madsister print --font …`) on top, by `Settings` key. */
export const mergeSettings = (saved: unknown, overrides: Record<string, unknown>): Settings => {
  const settings = parseSettings({ ...(typeof saved === "object" ? saved : {}), ...overrides });
  // parseSettings replaces an invalid value with the default: an override that didn't survive was invalid.
  for (const [key, value] of Object.entries(overrides)) {
    if (settings[key as keyof Settings] !== value) throw new Error(`invalid ${OPTIONS[key] ?? key}: ${JSON.stringify(value)}`);
  }
  return settings;
};

type FieldProps = {
  label: string;
  name?: string;
  value: string;
  type?: "text" | "number";
  /** Returns an error message, or null when the text is valid. */
  check?: (text: string) => string | null;
  onCommit: (text: string) => void;
};

/** A text input committed on Enter or blur (one history entry per change, not per keystroke). */
export const Field = ({ label, name, value, type = "text", check = () => null, onCommit }: FieldProps) => {
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
          name={name}
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

type Props = { history: ReturnType<typeof useHistory>; settings: Settings };

export const Toolbar = ({ history, settings }: Props) => {
  const [spelling, setSpelling] = useState<Spelling>("sharp");
  const hintId = useId();
  const { meta } = history.song;
  const setMeta = (patch: Partial<Song["meta"]>) => history.apply((s) => ({ ...s, meta: { ...s.meta, ...patch } }));
  // A key-less Nashville song keeps its chords relative to C: transposing would break that.
  const isTransposeLocked = meta.notation === "nashville" && meta.key === undefined;
  const offset = transposition(history.song); // net semitones since the song was opened, so Reset can undo them
  const transpose = (semitones: number) =>
    history.apply((s) => setTransposition(transposeSong(s, semitones, spelling), transposition(s) + semitones));
  return (
    <div className="toolbar">
      <Panel id="song">
        <Field label="Title" name="title" value={meta.title} check={(t) => (t === "" ? "a song needs a title" : null)} onCommit={(title) => setMeta({ title })} />
        <Field label="Artist" name="artist" value={meta.artist ?? ""} onCommit={(t) => setMeta({ artist: t || undefined })} />
        <Field
          label="Key"
          name="key"
          value={meta.key ?? ""}
          check={(t) => (t === "" || /^[A-G][#b]?m?$/.test(t) ? null : "a key is a note, optionally followed by m (e.g. F#m)")}
          onCommit={(t) => history.apply((s) => setKey(s, t || undefined, settings.minorConvention))}
        />
        <label>
          Notation{" "}
          <select
            value={meta.notation ?? "chords"}
            disabled={meta.key === undefined}
            aria-describedby={meta.key === undefined ? hintId : undefined}
            onChange={(e) => history.apply((s) => setNotation(s, e.target.value as Notation))}
          >
            <option value="chords">Chords</option>
            <option value="nashville">Nashville numbers</option>
          </select>
        </label>
        {meta.key === undefined && (
          <span id={hintId} className="field-hint">
            {meta.notation === "nashville" ? "Set a key to see the chords" : "Set a key to switch to Nashville numbers"}
          </span>
        )}
        <Field
          label="Tempo (BPM)"
          name="tempoBpm"
          type="number"
          value={meta.tempoBpm?.toString() ?? ""}
          check={(t) => (t === "" || Number(t) > 0 ? null : "the tempo must be a positive number")}
          onCommit={(t) => setMeta({ tempoBpm: t === "" ? undefined : Number(t) })}
        />
      </Panel>
      <Panel id="transpose" disabled={isTransposeLocked}>
        <label>
          Spelling{" "}
          <select value={spelling} onChange={(e) => setSpelling(e.target.value as Spelling)}>
            <option value="sharp">Sharps (♯)</option>
            <option value="flat">Flats (♭)</option>
          </select>
        </label>
        <div className="stepper">
          <button type="button" onClick={() => transpose(-1)}>
            −1 semitone
          </button>
          <output aria-live="polite">{offset > 0 ? `+${offset}` : offset}</output>
          <button type="button" onClick={() => transpose(1)}>
            +1 semitone
          </button>
        </div>
        <button type="button" disabled={offset % 12 === 0} onClick={() => transpose(-offset)}>
          Reset
        </button>
      </Panel>
    </div>
  );
};
