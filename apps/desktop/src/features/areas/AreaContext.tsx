import { createContext, useContext, type ReactNode } from 'react';

import type { Area } from '@/shared/api/types';

const OpenAreaContext = createContext<Area | undefined>(undefined);

interface OpenAreaProviderProps {
  /** The area that is open, or nothing when the whole project is shown. */
  area: Area | undefined;
  children: ReactNode;
}

/**
 * Makes the open area of the project on screen known to everything inside.
 * A task opened anywhere in a view then goes into that area, so that it
 * stays in sight, without every view having to know of areas.
 */
export function OpenAreaProvider({ area, children }: OpenAreaProviderProps) {
  return <OpenAreaContext value={area}>{children}</OpenAreaContext>;
}

/** The area that is open, or nothing when the whole project is shown. */
export function useOpenArea(): Area | undefined {
  return useContext(OpenAreaContext);
}
