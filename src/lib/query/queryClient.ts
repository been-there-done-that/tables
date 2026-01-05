import { QueryClient } from '@tanstack/svelte-query';

/**
 * Shared QueryClient instance for the application.
 * 
 * Configuration:
 * - staleTime: 60s - Data is considered fresh for 60 seconds
 * - gcTime: 5 minutes - Unused data is garbage collected after 5 minutes
 * - refetchOnWindowFocus: false - Don't refetch when window regains focus (desktop app)
 * - retry: 1 - Only retry failed requests once
 */
export const queryClient = new QueryClient({
    defaultOptions: {
        queries: {
            staleTime: 60_000, // 60 seconds
            gcTime: 5 * 60 * 1000, // 5 minutes (formerly cacheTime)
            refetchOnWindowFocus: false, // Desktop app, not needed
            retry: 1,
            refetchOnMount: false, // Use cached data when remounting
        },
    },
});
