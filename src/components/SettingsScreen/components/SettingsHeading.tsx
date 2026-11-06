import { PageHeader } from '@/components/PageHeader';
import type { ReactNode, Ref } from 'react';

export function SettingsHeading({
  heading,
  jobsControl,
  navigationControl,
}: {
  heading: Ref<HTMLHeadingElement>;
  jobsControl: ReactNode;
  navigationControl: ReactNode;
}) {
  return (
    <PageHeader>
      <header className="toolbar settings-heading">
        <div className="settings-title-row">
          {navigationControl}
          <h1 ref={heading} tabIndex={-1}>
            Settings
          </h1>
          {jobsControl}
        </div>
      </header>
    </PageHeader>
  );
}
