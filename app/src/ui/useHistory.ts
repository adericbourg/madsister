import { useReducer } from "react";
import * as history from "../model/history";
import type { Song } from "../model/song";

type Transition = (h: history.History) => history.History;

export const useHistory = (initial: Song) => {
  const [h, dispatch] = useReducer((state: history.History, t: Transition) => t(state), initial, history.reset);
  return {
    song: h.present,
    canUndo: history.canUndo(h),
    canRedo: history.canRedo(h),
    apply: (fn: (s: Song) => Song) => dispatch((state) => history.apply(state, fn)),
    undo: () => dispatch(history.undo),
    redo: () => dispatch(history.redo),
    reset: (song: Song) => dispatch(() => history.reset(song)),
  };
};
