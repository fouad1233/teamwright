import { CircleAlert, X } from 'lucide-react';
import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from 'react';

import { BackendError } from '@/shared/api/backend';
import { classNames } from '@/shared/lib/classNames';

import { Button, IconButton } from './Button';
import styles from './Toast.module.css';

/** How long a message stays on screen before it disappears by itself. */
const VISIBLE_FOR_MILLISECONDS = 7000;

/** Something the user can do about a message, such as showing a file. */
export interface NoticeAction {
  label: string;
  onSelect: () => void;
}

interface Notice {
  id: number;
  message: string;
  /** Whether the message says that something went wrong. */
  isFailure: boolean;
  action?: NoticeAction;
}

/** Tells the user about something that happened outside their view. */
export interface Notifier {
  /** Shows that something went wrong, in words the user can act on. */
  showError(message: string): void;
  /**
   * Shows that something the user asked for is done, such as a file that
   * was written, with one thing they can do about it.
   */
  showNotice(message: string, action?: NoticeAction): void;
}

const NotifierContext = createContext<Notifier | null>(null);

/** Shows short messages in a corner of the window. */
export function ToastProvider({ children }: { children: ReactNode }) {
  const [notices, setNotices] = useState<Notice[]>([]);
  const nextId = useRef(0);

  const dismiss = useCallback((id: number) => {
    setNotices((current) => current.filter((notice) => notice.id !== id));
  }, []);

  const show = useCallback(
    (notice: Omit<Notice, 'id'>) => {
      const id = nextId.current;
      nextId.current += 1;
      setNotices((current) => [...current, { id, ...notice }]);
      window.setTimeout(() => {
        dismiss(id);
      }, VISIBLE_FOR_MILLISECONDS);
    },
    [dismiss],
  );

  const notifier = useMemo<Notifier>(
    () => ({
      showError: (message) => {
        show({ message, isFailure: true });
      },
      showNotice: (message, action) => {
        show({ message, isFailure: false, action });
      },
    }),
    [show],
  );

  return (
    <NotifierContext value={notifier}>
      {children}
      <div className={styles.stack} role="status" aria-live="polite">
        {notices.map((notice) => (
          <div
            key={notice.id}
            className={classNames(styles.toast, notice.isFailure && styles.failure)}
          >
            {notice.isFailure && <CircleAlert className={styles.icon} />}
            <div className={styles.body}>
              <span className={styles.message} data-selectable>
                {notice.message}
              </span>
              {notice.action && (
                <Button
                  variant="ghost"
                  className={styles.action}
                  onClick={() => {
                    notice.action?.onSelect();
                    dismiss(notice.id);
                  }}
                >
                  {notice.action.label}
                </Button>
              )}
            </div>
            <IconButton
              label="Dismiss"
              onClick={() => {
                dismiss(notice.id);
              }}
            >
              <X />
            </IconButton>
          </div>
        ))}
      </div>
    </NotifierContext>
  );
}

/** The notifier of the window. */
export function useNotifier(): Notifier {
  const notifier = useContext(NotifierContext);
  if (!notifier) {
    throw new Error('useNotifier must be used inside a ToastProvider');
  }
  return notifier;
}

/**
 * Puts a failure into words for the user. The backend writes its messages as
 * sentence fragments ("the task title must not be empty"); this turns one
 * into a sentence.
 */
export function describeFailure(failure: unknown): string {
  if (failure instanceof BackendError && failure.message !== '') {
    const message = failure.message;
    return `${message.charAt(0).toUpperCase()}${message.slice(1)}.`;
  }
  return 'Something went wrong. Please try again.';
}
