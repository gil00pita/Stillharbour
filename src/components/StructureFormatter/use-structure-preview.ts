import { useEffect, useRef, useState } from 'react';
import type {
  LibraryOrganizationPreview,
  OrganizationPreviewQuery,
  OrganizationTemplate,
} from '../../bindings';
import { errorMessage, library, libraryData } from '../../ipc';

function discard(id: string | null) {
  if (id)
    void library({ kind: 'organization_library_discard', preview_id: id }).catch(() => {
      // Session TEMP data also expires on replacement and catalog close.
    });
}
export function useStructurePreview(revision: string) {
  const generation = useRef(0);
  const queries = useRef(0);
  const active = useRef<string | null>(null);
  const [preview, setPreview] = useState<LibraryOrganizationPreview | null>(null);
  const [buildingRequest, setBuildingRequest] = useState<number | null>(null);
  const [paging, setPaging] = useState(false);
  const [error, setError] = useState<string | null>(null);
  useEffect(
    () => () => {
      generation.current += 1;
      queries.current += 1;
      discard(active.current);
      active.current = null;
    },
    [],
  );
  function reset() {
    generation.current += 1;
    queries.current += 1;
    discard(active.current);
    active.current = null;
    setPreview(null);
    setBuildingRequest(null);
    setPaging(false);
    setError(null);
  }
  async function build(template: OrganizationTemplate) {
    reset();
    const request = generation.current;
    setBuildingRequest(request);
    try {
      let next = await libraryData(
        { kind: 'organization_library_start', template },
        'organization_library_preview',
      );
      if (request !== generation.current) {
        discard(next.previewId);
        return false;
      }
      active.current = next.previewId;
      setPreview(next);
      while (!next.complete) {
        next = await libraryData(
          { kind: 'organization_library_advance', preview_id: next.previewId },
          'organization_library_preview',
        );
        if (request !== generation.current) return false;
        setPreview(next);
      }
      return true;
    } catch (cause) {
      if (request === generation.current) {
        setError(errorMessage(cause));
        setPreview(null);
        discard(active.current);
        active.current = null;
      }
      return false;
    } finally {
      if (request === generation.current) {
        setBuildingRequest(null);
      }
    }
  }
  async function page(offset: number, query: OrganizationPreviewQuery) {
    if (!preview?.complete || preview.catalogRevision !== revision) return;
    const request = ++queries.current;
    setPaging(true);
    setError(null);
    try {
      const next = await libraryData(
        { kind: 'organization_library_query', preview_id: preview.previewId, offset, query },
        'organization_library_preview',
      );
      if (request === queries.current) setPreview(next);
    } catch (cause) {
      if (request === queries.current) setError(errorMessage(cause));
    } finally {
      if (request === queries.current) setPaging(false);
    }
  }
  async function validate() {
    if (!preview?.complete || preview.catalogRevision !== revision)
      throw new Error('The catalog has changed. Return to editing and build a fresh preview.');
    return libraryData(
      { kind: 'organization_library_page', preview_id: preview.previewId, offset: 0 },
      'organization_library_preview',
    );
  }
  return {
    preview,
    busy: buildingRequest !== null,
    paging,
    error,
    stale: preview !== null && preview.catalogRevision !== revision,
    build,
    page,
    reset,
    validate,
  };
}
