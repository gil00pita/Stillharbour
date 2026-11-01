import type { CopyWorkflowJob } from '../bindings';
export function copyJobActive(job: CopyWorkflowJob) {
  return ['queued', 'running', 'pause_requested', 'cancel_requested'].includes(job.state);
}
