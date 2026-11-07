import { Box, Heading, chakra } from '@chakra-ui/react';
import { useEffect, useState, type ReactNode } from 'react';
import { Button } from '@/components/Button';

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
      <chakra.header>
        <Heading as="h2" unstyled>
          {title}
        </Heading>
        <Button onClick={close}>Close</Button>
      </chakra.header>
      <Box className="duplicate-dialog-body">{children}</Box>
    </dialog>
  );
}
