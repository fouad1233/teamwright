import { isTauri } from '@tauri-apps/api/core';
import { createContext, useContext, type ReactNode } from 'react';

import type { Backend } from './backend';
import { DemoBackend } from './demo/demoBackend';
import { sampleWorkspace } from './demo/sampleWorkspace';
import { TauriBackend } from './tauriBackend';

/** The backend the interface works with, and whether it is the demo. */
export interface BackendConnection {
  backend: Backend;
  /** True when the interface runs in a browser with sample data. */
  isDemo: boolean;
}

/**
 * Chooses the backend: the desktop shell when the interface runs inside it,
 * and the in-browser demo otherwise.
 *
 * The demo starts with a sample project. Opening it with `?demo=empty` in
 * the address starts it with nothing, which shows the first screen a new
 * user sees.
 */
export function connectToBackend(): BackendConnection {
  if (isTauri()) {
    return { backend: new TauriBackend(), isDemo: false };
  }
  const startsEmpty = new URLSearchParams(window.location.search).get('demo') === 'empty';
  const workspace = startsEmpty ? undefined : sampleWorkspace();
  const demo = new DemoBackend(workspace);
  // Scenarios that take pictures of the demo reach it here, to show what a
  // browser cannot make happen, such as a sync folder that is gone.
  window.teamwrightDemo = demo;
  return { backend: demo, isDemo: true };
}

declare global {
  interface Window {
    /** The demo backend, while the interface runs in a browser on sample data. */
    teamwrightDemo?: DemoBackend;
  }
}

const BackendContext = createContext<BackendConnection | null>(null);

interface BackendProviderProps {
  connection: BackendConnection;
  children: ReactNode;
}

/** Makes the backend available to every component below it. */
export function BackendProvider({ connection, children }: BackendProviderProps) {
  return <BackendContext value={connection}>{children}</BackendContext>;
}

function useBackendConnection(): BackendConnection {
  const connection = useContext(BackendContext);
  if (!connection) {
    throw new Error('useBackend must be used inside a BackendProvider');
  }
  return connection;
}

/** The backend to send requests to. */
export function useBackend(): Backend {
  return useBackendConnection().backend;
}

/** Whether the interface runs as a demo with sample data. */
export function useIsDemo(): boolean {
  return useBackendConnection().isDemo;
}
