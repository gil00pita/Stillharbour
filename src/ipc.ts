import { unpackPage } from './pageTransport';
import { diagnosticClock, diagnosticPayload, diagnosticSince } from './performance';
import { invoke, isTauri } from '@tauri-apps/api/core';
import {
  FOUNDATION_COMMAND,
  LIBRARY_COMMAND,
  type LibraryRequest,
  type LibraryResponse as NativeLibraryResponse,
  type AssetPage,
  type AppError,
  type DirectorySelection,
  type FoundationResponse,
  type FoundationStatus,
  type FoundationRequest,
} from './bindings';

async function request(request: FoundationRequest): Promise<FoundationResponse> {
  if (!isTauri()) {
    throw new Error('Open the Stillharbour desktop app to connect to your local catalog.');
  }
  // The pinned WDIO plugin's global interception does not wrap imported API
  // calls. This opt-in fixture route is compiled out of the normal app.
  if (import.meta.env.MODE === 'desktop-test') {
    const fixture = (
      window as Window & {
        __wdio_mocks__?: Record<
          string,
          (args: { request: FoundationRequest }) => FoundationResponse | Promise<FoundationResponse>
        >;
      }
    ).__wdio_mocks__?.[FOUNDATION_COMMAND];
    if (fixture) return fixture({ request });
  }
  return invoke<FoundationResponse>(FOUNDATION_COMMAND, { request });
}

export async function getStatus(): Promise<FoundationStatus> {
  const response = await request({ kind: 'status' });
  if (response.kind !== 'status') throw new Error('Unexpected native response. Reopen the app.');
  return response.data;
}

export async function pickDirectory(): Promise<DirectorySelection | null> {
  const response = await request({ kind: 'pick_directory' });
  if (response.kind !== 'directory') throw new Error('Unexpected picker response. Try again.');
  return response.data;
}

export function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (
    typeof error === 'object' &&
    error !== null &&
    'message' in error &&
    typeof error.message === 'string'
  ) {
    return (error as Pick<AppError, 'message'>).message;
  }
  return 'The native connection failed. Reopen the app and try again.';
}

type LibraryResponse =
  Exclude<NativeLibraryResponse, { kind: 'page' }> | { kind: 'page'; data: AssetPage };

let thumbnailGeneration = 0n;
export function nextThumbnailGeneration(): string {
  thumbnailGeneration += 1n;
  return String(thumbnailGeneration);
}

export async function library(request: LibraryRequest): Promise<LibraryResponse> {
  if (!isTauri())
    throw new Error('Open the Stillharbour desktop app to connect to your local catalog.');
  if (import.meta.env.MODE === 'desktop-test')
    window.dispatchEvent(new CustomEvent('stillharbour:test-library-request', { detail: request }));
  const started = diagnosticClock();
  diagnosticPayload('ipc_request_bytes', request);
  let response: NativeLibraryResponse;
  try {
    response = await invoke<NativeLibraryResponse>(LIBRARY_COMMAND, { request });
    diagnosticPayload('ipc_response_bytes', response);
  } finally {
    diagnosticSince('ipc_ms', started);
  }
  if (response.kind === 'status')
    thumbnailGeneration =
      thumbnailGeneration > BigInt(response.data.previewGeneration)
        ? thumbnailGeneration
        : BigInt(response.data.previewGeneration);
  return response.kind === 'page' ? { kind: 'page', data: unpackPage(response.data) } : response;
}

type DataKind = Exclude<LibraryResponse['kind'], 'done'>;
type ResponseData<K extends DataKind> =
  Extract<LibraryResponse, { kind: K }> extends { data: infer D } ? D : never;
export async function libraryData<K extends DataKind>(
  request: LibraryRequest,
  kind: K,
): Promise<ResponseData<K>> {
  const response = await library(request);
  if (response.kind !== kind || !('data' in response))
    throw new Error('Unexpected library response. Refresh the library.');
  return response.data as ResponseData<K>;
}

export async function getPreferences(): Promise<import('./bindings').AppPreferences> {
  const response = await request({ kind: 'get_preferences' });
  if (response.kind !== 'preferences') throw new Error('Unexpected settings response. Try again.');
  return response.data;
}
export async function savePreferences(
  preferences: import('./bindings').AppPreferences,
): Promise<import('./bindings').AppPreferences> {
  const response = await request({ kind: 'save_preferences', preferences });
  if (response.kind !== 'preferences')
    throw new Error('Settings save was not confirmed. Try again.');
  return response.data;
}
