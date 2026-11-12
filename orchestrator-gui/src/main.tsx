import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { BrowserRouter } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import App from './App';

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      // The health dashboard polls, so a short stale time keeps the UI responsive without
      // hammering the API.
      staleTime: 5_000,
      retry: 1,
      refetchOnWindowFocus: true,
    },
  },
});

const container = document.getElementById('root');
if (!container) {
  throw new Error('the #root element is missing from index.html');
}

createRoot(container).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <BrowserRouter>
        <App />
      </BrowserRouter>
    </QueryClientProvider>
  </StrictMode>,
);
