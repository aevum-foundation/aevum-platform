/**
 * AEVUM WEB PLATFORM
 * STORE LAYER v3
 *
 * Architecture:
 *   UI → Store → AevumAPI → Provider
 *
 * Responsibilities:
 *   - keyed response cache
 *   - TTL management
 *   - request deduplication
 *   - loading/error state
 *   - reactive subscriptions
 *   - controlled polling
 *
 * IMPORTANT:
 * - Store never knows whether provider is mock or network.
 * - Cache keys are parameter-sensitive.
 * - lastUpdated means last successful data update.
 * - Internal cache entries are never exposed for mutation.
 */

import { AevumAPI } from './client.js';

// ============================================================
// CACHE TTL
// ============================================================

const CACHE_TTL = Object.freeze({
    network: 30_000,
    epoch: 30_000,
    epochDetail: 30_000,
    supply: 60_000,
    participants: 60_000,
    activity: 30_000,
    transaction: 60_000,
    address: 60_000,
    search: 10_000,
});

const DEFAULT_POLL_INTERVAL = 30_000;
const MIN_POLL_INTERVAL = 5_000;

// ============================================================
// INTERNAL STATE
// ============================================================

const cache = new Map();
const subscribers = new Map();
const pendingRequests = new Map();

let pollingTimer = null;
let pollingInterval = DEFAULT_POLL_INTERVAL;

// ============================================================
// CACHE HELPERS
// ============================================================

function createEmptyEntry() {
    return {
        data: null,
        loading: false,
        error: null,
        lastUpdated: null,
    };
}

function getCacheKey(namespace, ...parts) {
    const normalized = parts.map(part => String(part));
    return [namespace, ...normalized].join(':');
}

function getEntry(key) {
    let entry = cache.get(key);

    if (!entry) {
        entry = createEmptyEntry();
        cache.set(key, entry);
    }

    return entry;
}

function isFresh(entry, ttl) {
    if (!entry || entry.lastUpdated === null) {
        return false;
    }

    return (Date.now() - entry.lastUpdated) < ttl;
}

function cloneEntry(entry) {
    return {
        data: entry.data,
        loading: entry.loading,
        error: entry.error,
        lastUpdated: entry.lastUpdated,
    };
}

function notify(key) {
    const callbacks = subscribers.get(key);

    if (!callbacks || callbacks.size === 0) {
        return;
    }

    const entry = cloneEntry(getEntry(key));

    for (const callback of callbacks) {
        try {
            callback(entry);
        } catch (error) {
            console.error(
                `[Store] Subscriber error for ${key}:`,
                error
            );
        }
    }
}

function updateEntry(key, updates, { touch = false } = {}) {
    const entry = getEntry(key);

    Object.assign(entry, updates);

    if (touch) {
        entry.lastUpdated = Date.now();
    }

    notify(key);
}

function setLoading(key) {
    updateEntry(key, {
        loading: true,
        error: null,
    });
}

function setData(key, data) {
    updateEntry(
        key,
        {
            data,
            loading: false,
            error: null,
        },
        { touch: true }
    );
}

function setError(key, error) {
    updateEntry(key, {
        loading: false,
        error: error instanceof Error
            ? error.message
            : String(error),
    });
}

// ============================================================
// CORE FETCH
// ============================================================

async function fetchCached(
    key,
    ttl,
    loader,
    { force = false } = {}
) {
    const entry = getEntry(key);

    if (
        !force &&
        entry.data !== null &&
        isFresh(entry, ttl)
    ) {
        return entry.data;
    }

    const pending = pendingRequests.get(key);

    if (pending) {
        return pending;
    }

    setLoading(key);

    const request = Promise.resolve()
        .then(loader)
        .then(data => {
            setData(key, data);
            return data;
        })
        .catch(error => {
            setError(key, error);
            throw error;
        })
        .finally(() => {
            pendingRequests.delete(key);
        });

    pendingRequests.set(key, request);

    return request;
}

// ============================================================
// INPUT VALIDATION
// ============================================================

function requireNonEmptyString(value, fieldName) {
    if (
        typeof value !== 'string' ||
        value.trim().length === 0
    ) {
        throw new TypeError(
            `${fieldName} must be a non-empty string`
        );
    }

    return value.trim();
}

function normalizePositiveInteger(value, fieldName, fallback) {
    if (value === undefined) {
        return fallback;
    }

    const number = Number(value);

    if (
        !Number.isSafeInteger(number) ||
        number < 0
    ) {
        throw new TypeError(
            `${fieldName} must be a non-negative safe integer`
        );
    }

    return number;
}

// ============================================================
// PUBLIC API
// ============================================================

export const Store = Object.freeze({

    // ==========================================================
    // NETWORK
    // ==========================================================

    async getNetworkStatus(options = {}) {
        return fetchCached(
            getCacheKey('network'),
            CACHE_TTL.network,
            () => AevumAPI.getNetworkStatus(),
            options
        );
    },

    async getSupply(options = {}) {
        return fetchCached(
            getCacheKey('supply'),
            CACHE_TTL.supply,
            () => AevumAPI.getSupply(),
            options
        );
    },

    // ==========================================================
    // EPOCH
    // ==========================================================

    async getEpoch(epochId, options = {}) {
        const id = normalizePositiveInteger(
            epochId,
            'Epoch ID',
            undefined
        );

        const key = getCacheKey('epoch', id);

        return fetchCached(
            key,
            CACHE_TTL.epoch,
            () => AevumAPI.getEpoch(id),
            options
        );
    },

    async getEpochDetail(epochId, options = {}) {
        const id = normalizePositiveInteger(
            epochId,
            'Epoch ID',
            undefined
        );

        const key = getCacheKey('epochDetail', id);

        return fetchCached(
            key,
            CACHE_TTL.epochDetail,
            () => AevumAPI.getEpochDetail(id),
            options
        );
    },

    // ==========================================================
    // PARTICIPANTS
    // ==========================================================

    async getParticipants(
        limit = 10,
        offset = 0,
        options = {}
    ) {
        const normalizedLimit = normalizePositiveInteger(
            limit,
            'Limit',
            10
        );

        const normalizedOffset = normalizePositiveInteger(
            offset,
            'Offset',
            0
        );

        const key = getCacheKey(
            'participants',
            normalizedLimit,
            normalizedOffset
        );

        return fetchCached(
            key,
            CACHE_TTL.participants,
            () => AevumAPI.getParticipants(
                normalizedLimit,
                normalizedOffset
            ),
            options
        );
    },

    // ==========================================================
    // ACTIVITY
    // ==========================================================

    async getRecentActivity(
        limit = 10,
        options = {}
    ) {
        const normalizedLimit = normalizePositiveInteger(
            limit,
            'Limit',
            10
        );

        const key = getCacheKey(
            'activity',
            normalizedLimit
        );

        return fetchCached(
            key,
            CACHE_TTL.activity,
            () => AevumAPI.getRecentActivity(
                normalizedLimit
            ),
            options
        );
    },

    // ==========================================================
    // TRANSACTIONS
    // ==========================================================

    async getTransactionDetail(
        txHash,
        options = {}
    ) {
        const normalizedHash = requireNonEmptyString(
            txHash,
            'Transaction hash'
        ).toLowerCase();

        const key = getCacheKey(
            'transaction',
            normalizedHash
        );

        return fetchCached(
            key,
            CACHE_TTL.transaction,
            () => AevumAPI.getTransactionDetail(
                normalizedHash
            ),
            options
        );
    },

    // ==========================================================
    // ADDRESSES
    // ==========================================================

    async getAddressDetail(
        address,
        options = {}
    ) {
        const normalizedAddress = requireNonEmptyString(
            address,
            'Address'
        ).toLowerCase();

        const key = getCacheKey(
            'address',
            normalizedAddress
        );

        return fetchCached(
            key,
            CACHE_TTL.address,
            () => AevumAPI.getAddressDetail(
                normalizedAddress
            ),
            options
        );
    },

    // ==========================================================
    // SEARCH
    // ==========================================================

    async search(query, options = {}) {
        if (typeof query !== 'string') {
            return null;
        }

        const normalized = query.trim().toLowerCase();

        if (normalized.length < 2) {
            return null;
        }

        const key = getCacheKey(
            'search',
            normalized
        );

        return fetchCached(
            key,
            CACHE_TTL.search,
            () => AevumAPI.search(normalized),
            options
        );
    },

    // ==========================================================
    // STATE
    // ==========================================================

    getState(key) {
        if (key !== undefined) {
            const entry = cache.get(key);

            return entry
                ? cloneEntry(entry)
                : null;
        }

        const result = {};

        for (const [cacheKey, entry] of cache) {
            result[cacheKey] = cloneEntry(entry);
        }

        return result;
    },

    // ==========================================================
    // CACHE MANAGEMENT
    // ==========================================================

    clearCache(key) {
        if (key !== undefined) {
            cache.delete(key);

            /*
             * Notify subscribers without recreating
             * the deleted cache entry.
             */
            const callbacks = subscribers.get(key);

            if (!callbacks || callbacks.size === 0) {
                return;
            }

            const emptyEntry = createEmptyEntry();

            for (const callback of callbacks) {
                try {
                    callback(emptyEntry);
                } catch (error) {
                    console.error(
                        `[Store] Subscriber error for ${key}:`,
                        error
                    );
                }
            }

            return;
        }

        const keys = Array.from(cache.keys());

        cache.clear();

        for (const cacheKey of keys) {
            const callbacks = subscribers.get(cacheKey);

            if (!callbacks || callbacks.size === 0) {
                continue;
            }

            const emptyEntry = createEmptyEntry();

            for (const callback of callbacks) {
                try {
                    callback(emptyEntry);
                } catch (error) {
                    console.error(
                        `[Store] Subscriber error for ${cacheKey}:`,
                        error
                    );
                }
            }
        }
    },

    // ==========================================================
    // SUBSCRIPTIONS
    // ==========================================================

    subscribe(key, callback) {
        if (
            typeof key !== 'string' ||
            key.length === 0
        ) {
            throw new TypeError(
                'Subscription key must be a non-empty string'
            );
        }

        if (typeof callback !== 'function') {
            throw new TypeError(
                'Subscription callback must be a function'
            );
        }

        if (!subscribers.has(key)) {
            subscribers.set(key, new Set());
        }

        const callbacks = subscribers.get(key);
        callbacks.add(callback);

        try {
            callback(cloneEntry(getEntry(key)));
        } catch (error) {
            console.error(
                `[Store] Subscriber error for ${key}:`,
                error
            );
        }

        return () => {
            const current = subscribers.get(key);

            if (!current) {
                return;
            }

            current.delete(callback);

            if (current.size === 0) {
                subscribers.delete(key);
            }
        };
    },

    // ==========================================================
    // POLLING
    // ==========================================================

    startPolling(interval = DEFAULT_POLL_INTERVAL) {
        const normalizedInterval = Number(interval);

        if (
            !Number.isFinite(normalizedInterval) ||
            normalizedInterval < MIN_POLL_INTERVAL
        ) {
            throw new TypeError(
                `Polling interval must be at least ${MIN_POLL_INTERVAL}ms`
            );
        }

        if (pollingTimer) {
            return false;
        }

        pollingInterval = normalizedInterval;

        const poll = () => {
            if (
                typeof document !== 'undefined' &&
                document.visibilityState === 'hidden'
            ) {
                return;
            }

            this.getNetworkStatus({ force: true })
                .catch(() => {});

            this.getRecentActivity(10, { force: true })
                .catch(() => {});

            this.getSupply({ force: true })
                .catch(() => {});
        };

        pollingTimer = setInterval(
            poll,
            pollingInterval
        );

        return true;
    },

    stopPolling() {
        if (!pollingTimer) {
            return false;
        }

        clearInterval(pollingTimer);
        pollingTimer = null;

        return true;
    },

    isPolling() {
        return pollingTimer !== null;
    },

    getPollingInterval() {
        return pollingInterval;
    },
});

export default Store;
