import { Tooltip as TooltipPrimitive } from 'radix-ui';
import type { ReactElement, ReactNode } from 'react';

import styles from './Tooltip.module.css';

/** How long the pointer rests on a control before its tooltip appears. */
const DELAY_MILLISECONDS = 450;

/** Lets every tooltip below it appear after the same short delay. */
export function TooltipProvider({ children }: { children: ReactNode }) {
  return (
    <TooltipPrimitive.Provider delayDuration={DELAY_MILLISECONDS} skipDelayDuration={200}>
      {children}
    </TooltipPrimitive.Provider>
  );
}

interface TooltipProps {
  /** What the control does, in a few words. */
  label: string;
  /** The control the tooltip belongs to. */
  children: ReactElement;
  side?: 'top' | 'right' | 'bottom' | 'left';
}

/** A short label that appears next to a control on hover and on focus. */
export function Tooltip({ label, children, side = 'bottom' }: TooltipProps) {
  return (
    <TooltipPrimitive.Root>
      <TooltipPrimitive.Trigger asChild>{children}</TooltipPrimitive.Trigger>
      <TooltipPrimitive.Portal>
        <TooltipPrimitive.Content className={styles.tooltip} side={side} sideOffset={6}>
          {label}
        </TooltipPrimitive.Content>
      </TooltipPrimitive.Portal>
    </TooltipPrimitive.Root>
  );
}
