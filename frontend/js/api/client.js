// Aevum Web — Provider Client
// Canonical provider boundary.
// IMPORTANT:
// - mock is explicit demo mode only
// - network never silently falls back to mock
// - auto never silently falls back to mock
// - all canonical API methods live inside one object

import MockProvider from './mock.js';

const CONFIG = Object.freeze({
    /**
     * 'mock' | 'network' | 'auto'
     *
     * Production must use 'network'.
     * Mock is only for explicit demo/testing.
     */
    provider: 'mock',

    /** Base URL for NetworkProvider. */
    apiBaseUrl: 'https://aevumchain.com',

    /** Request timeout in milliseconds. */
    requestTimeout: 10000,

    /** Provider health-check interval in milliseconds. */
    healthCheckInterval: 30000,

    /** Enable client diagnostics. */
    debug: false,
});


class ProviderManager {

    #provider = null;
    #providerName = null;

    constructor() {
        this.#provider = this.#selectProvider();
        this.#providerName = this.#provider?.constructor?.name || 'UnknownProvider';

        if (CONFIG.debug) {
            console.info(`[AevumAPI] Provider: ${this.#providerName}`);
        }
    }

    /**
     * Select provider.
     *
     * IMPORTANT:
     * Network mode MUST fail when NetworkProvider is unavailable.
     * It must never silently use MockProvider.
     */
    #selectProvider() {
        const mode = CONFIG.provider;

        if (mode === 'mock') {
            return MockProvider;
        }

        if (mode === 'network') {
            throw new Error(
                'NetworkProvider is not configured. ' +
                'Refusing to fall back to MockProvider.'
            );
        }

        if (mode === 'auto') {
            throw new Error(
                'No production NetworkProvider is registered. ' +
                'Refusing to fall back to MockProvider.'
            );
        }

        throw new Error(`Unknown Aevum provider mode: ${mode}`);
    }

    /**
     * Get active provider.
     */
    get provider() {
        if (!this.#provider) {
            throw new Error('Aevum provider is not available');
        }

        return this.#provider;
    }

    /**
     * Provider health check.
     *
     * Providers may expose an optional health() method.
     */
    async healthCheck() {
        const provider = this.provider;

        if (typeof provider.health !== 'function') {
            return true;
        }

        await provider.health();
        return true;
    }

    /**
     * Provider name.
     */
    get providerName() {
        return this.#providerName;
    }

    /**
     * Current configuration.
     *
     * Returned as a copy so callers cannot mutate internal config.
     */
    get config() {
        return { ...CONFIG };
    }
}


const manager = new ProviderManager();


/**
 * Canonical frontend API.
 *
 * Every method delegates directly to the selected provider.
 */
export const AevumAPI = Object.freeze({

    // ==========================================================
    // NETWORK
    // ==========================================================

    getNetworkStatus: () =>
        manager.provider.getNetworkStatus(),

    getSupply: () =>
        manager.provider.getSupply(),

    // ==========================================================
    // EPOCH / FINALITY
    // ==========================================================

    getEpoch: (epochId) =>
        manager.provider.getEpoch(epochId),

    getEpochDetail: (epochId) =>
        manager.provider.getEpochDetail(epochId),

    // ==========================================================
    // PARTICIPANTS
    // ==========================================================

    getParticipants: (limit, offset) =>
        manager.provider.getParticipants(limit, offset),

    // ==========================================================
    // ACTIVITY
    // ==========================================================

    getRecentActivity: (limit) =>
        manager.provider.getRecentActivity(limit),

    // ==========================================================
    // TRANSACTIONS
    // ==========================================================

    getTransactionDetail: (txHash) =>
        manager.provider.getTransactionDetail(txHash),

    // ==========================================================
    // ADDRESSES
    // ==========================================================

    getAddressDetail: (address) =>
        manager.provider.getAddressDetail(address),

    // ==========================================================
    // SEARCH
    // ==========================================================

    search: (query) =>
        manager.provider.search(query),
});


/**
 * Client diagnostics / metadata.
 *
 * These are deliberately separate from the canonical API contract.
 */
export const AevumClient = Object.freeze({

    getProviderName: () =>
        manager.providerName,

    getConfig: () =>
        manager.config,

    healthCheck: () =>
        manager.healthCheck(),
});


/**
 * Optional browser globals.
 *
 * Exposed only when debug mode is explicitly enabled.
 */
if (CONFIG.debug && typeof window !== 'undefined') {
    window.AevumAPI = AevumAPI;
    window.AevumClient = AevumClient;
}


export default AevumAPI;
