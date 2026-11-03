import { createContext } from 'react';
import type { AssetNotes } from '../bindings';

export type NotesDraft = { baseline: AssetNotes; caption: string; rejected: boolean };
// Owned by the open catalog's UI session, never shared across catalogs or persisted as a save.
export const NotesDraftContext = createContext<Map<string, NotesDraft> | null>(null);
export const MAX_NOTES_DRAFTS = 240;
