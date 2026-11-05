import { useEffect, useState, type ReactNode } from 'react';
import { Button } from '../../ui/controls';

export function ReviewDialog({
  title,
  onClose,
  children,
}: {
  title: string;
  onClose: () => void;
  children: ReactNode;
}) {
  const [element, setElement] = useState<HTMLDialogElement | null>(null);
  useEffect(() => {
    if (element && !element.open) element.showModal();
    return () => element?.close();
  }, [element]);
  function close() {
    element?.close();
    onClose();
  }
  return (
    <dialog
      ref={setElement}
      className="duplicate-review-dialog"
      aria-label={title}
      onCancel={(event) => {
        event.preventDefault();
        close();
      }}
    >
      <header>
        <h2>{title}</h2>
        <Button onClick={close}>Close</Button>
      </header>
      <div className="duplicate-dialog-body">{children}</div>
    </dialog>
  );
}
