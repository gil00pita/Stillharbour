import { Heading, Text, chakra } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { useEffect, useRef, useState } from 'react';
import type { ImportCopyReceipt, ImportOutputIndex, ScanJob } from '../../bindings';
import { errorMessage, library, libraryData, pickDirectory } from '../../ipc';
import { ScanJobProgress } from '@/components/ScanJobProgress';

const active = (job: ScanJob | null) =>
  !!job && ['queued', 'running', 'cancel_requested'].includes(job.state);
export function ImportOutputIndexReview({
  receipt,
  sourceRootId,
}: {
  receipt: ImportCopyReceipt;
  sourceRootId: string;
}) {
  const [result, setResult] = useState<ImportOutputIndex | null>(null);
  const [scan, setScan] = useState<ScanJob | null>(null);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const epoch = useRef(0);
  useEffect(
    () => () => {
      epoch.current += 1;
    },
    [],
  );
  function validate(value: ImportOutputIndex) {
    if (
      value.jobId !== receipt.jobId ||
      !value.root.id ||
      value.root.id === sourceRootId ||
      (receipt.skippedFiles.some((file) => file.keeperRootId === value.root.id) &&
        !(value.alreadyRegistered && value.root.mode === 'managed')) ||
      value.originalMutationsEnabled ||
      (!value.alreadyRegistered && value.root.mode !== 'referenced') ||
      (value.scan && (value.scan.rootId !== value.root.id || value.scan.kind === 'thumbnails'))
    )
      throw new Error('The output registration does not match this verified import.');
  }
  async function index() {
    const token = ++epoch.current;
    setPending(true);
    setError(null);
    try {
      const chosen = await pickDirectory();
      if (epoch.current !== token || !chosen) return;
      if (!chosen.identityAvailable)
        throw new Error('The selected destination identity cannot be verified.');
      const value = await libraryData(
        {
          kind: 'organization_import_index_output',
          job_id: receipt.jobId,
          selection_id: chosen.id,
        },
        'organization_import_output_index',
      );
      validate(value);
      if (epoch.current !== token) {
        if ((value.scanStarted ?? !value.alreadyRegistered) && value.scan && active(value.scan))
          void library({ kind: 'cancel', job_id: value.scan.id }).catch(() => {});
        return;
      }
      setResult(value);
      setScan(value.scan);
    } catch (error) {
      if (epoch.current === token) setError(errorMessage(error));
    } finally {
      if (epoch.current === token) setPending(false);
    }
  }
  const scanId = scan?.id;
  const rootId = result?.root.id;
  const running = active(scan);
  useEffect(() => {
    if (!running || !scanId || !rootId) return;
    let live = true;
    let timer: ReturnType<typeof setTimeout>;
    async function poll() {
      try {
        const value = await libraryData({ kind: 'job_details', job_id: scanId! }, 'job_details');
        if (!live) return;
        if (
          value.job.id !== scanId ||
          value.job.rootId !== rootId ||
          value.job.kind === 'thumbnails'
        )
          throw new Error('The scan belongs to a different folder.');
        setScan(value.job);
        if (active(value.job)) timer = setTimeout(() => void poll(), 500);
      } catch (error) {
        if (live) {
          setError(errorMessage(error));
          timer = setTimeout(() => void poll(), 1000);
        }
      }
    }
    timer = setTimeout(() => void poll(), 100);
    return () => {
      live = false;
      clearTimeout(timer);
    };
  }, [running, scanId, rootId]);
  async function cancel() {
    if (!scan) return;
    setPending(true);
    setError(null);
    try {
      await library({ kind: 'cancel', job_id: scan.id });
    } catch (error) {
      setError(errorMessage(error));
    } finally {
      setPending(false);
    }
  }
  return (
    <chakra.section aria-label="Index import output" aria-busy={pending}>
      <Heading as="h4" unstyled>
        Add the output folder to your catalog
      </Heading>
      <Text>
        Choose the original destination again. Stillharbour scans this completed output folder
        without changing files. New folders are registered as referenced libraries; an explicitly
        selected managed destination uses its existing library scan settings. Indexing is separate
        from byte verification and independent backup.
      </Text>
      <Button type="button" disabled={pending || running} onClick={() => void index()}>
        {result ? 'Review output registration' : 'Choose destination and index output'}
      </Button>
      {result && (
        <Text role="status">
          {result.alreadyRegistered && result.root.mode === 'managed'
            ? 'Using the existing managed destination library'
            : result.alreadyRegistered
              ? 'Output folder already registered'
              : 'Output folder registered'}
          : <bdi>{result.root.displayPath}</bdi>.
        </Text>
      )}
      {result && !scan && (
        <Text>
          No scan is retained in the recent job list. Start a scan from the folder settings.
        </Text>
      )}
      {scan && (
        <>
          <ScanJobProgress job={scan} label={result?.root.label ?? 'Imported output'} />
          <Text>
            {scan.indexed} indexed entries · {scan.errors} issues.
          </Text>
        </>
      )}
      {running && (
        <Button type="button" disabled={pending} onClick={() => void cancel()}>
          Cancel output scan
        </Button>
      )}
      {error && <Text role="alert">{error}</Text>}
    </chakra.section>
  );
}
