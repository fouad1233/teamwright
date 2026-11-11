import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';

import { App } from '@/app/App';
import { NavigationProvider } from '@/app/navigation';
import { BackendProvider, connectToBackend } from '@/shared/api/BackendProvider';
import { ToastProvider } from '@/shared/ui/Toast';

import '@/shared/styles/tokens.css';
import '@/shared/styles/base.css';

const connection = connectToBackend();

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      // The backend is on this computer. A failed request will fail again,
      // and nothing changes behind the interface's back while the window is
      // out of focus.
      retry: false,
      refetchOnWindowFocus: false,
    },
  },
});

const rootElement = document.getElementById('root');
if (!rootElement) {
  throw new Error('index.html has no element with the id "root"');
}

createRoot(rootElement).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <BackendProvider connection={connection}>
        <ToastProvider>
          <NavigationProvider>
            <App />
          </NavigationProvider>
        </ToastProvider>
      </BackendProvider>
    </QueryClientProvider>
  </StrictMode>,
);
