/**
 * Global Application Constants
 * 
 * Centralized configuration for use across the application.
 */

export const APP = {
    NAME: 'Tables',
    VERSION: '0.1.0'
} as const;

export const METRICS = {
    /** Feature flag to enable metrics UI/IPC */
    ENABLED: false,
    /** Number of historical samples to keep for sparklines */
    HISTORY_SIZE: 20,
    /** Default height for the sparkline in pixels */
    CHART_HEIGHT: 26,
    /** Default bar width in pixels */
    BAR_WIDTH: 4,
} as const;
