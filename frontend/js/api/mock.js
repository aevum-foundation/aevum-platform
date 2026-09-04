/**
 * AEVUM WEB PLATFORM
 * MOCK PROVIDER v3.0
 *
 * Deterministic reference implementation of the Aevum API contract.
 *
 * Architecture:
 *
 *   UI
 *    ↓
 *   Store
 *    ↓
 *   AevumAPI
 *    ↓
 *   MockProvider
 *
 * This provider is DEMO/TEST ONLY.
 *
 * Canonical monetary assumptions:
 *   Genesis supply       = 21,000,000 AEV
 *   Emission budget      = 350,000,000 AEV
 *   Maximum supply       = 371,000,000 AEV
 *   Epoch                = 2,880 slots
 *   Slot                 = 30 seconds
 *   Halving interval     = 1,461 epochs
 *
 * Determinism guarantees:
 *   - No Math.random()
 *   - No Date.now()
 *   - No performance.now()
 *   - Fixed reference time
 *   - Same input + same provider version = same result
 */

import { AevumAPI } from './contract.js';

// ============================================================
// MONETARY CONSTITUTION
// ============================================================

const GENESIS_SUPPLY = 21_000_000;
const EMISSION_BUDGET = 350_000_000;
const MAX_SUPPLY = GENESIS_SUPPLY + EMISSION_BUDGET;

const EPOCH_SLOTS = 2_880;
const SLOT_DURATION_SECONDS = 30;
const EPOCH_DURATION_SECONDS = EPOCH_SLOTS * SLOT_DURATION_SECONDS;

const EPOCHS_PER_HALVING = 1_461;

const EMISSION_SCHEDULE = Object.freeze([
    20,
    10,
    5,
    2.5,
    1.25,
]);

// ============================================================
// MOCK NETWORK STATE
// ============================================================

const MOCK_EPOCH = 142;
const MOCK_SLOT = 1_834;
const MOCK_PARTICIPANTS = 1_247;

const state = Object.freeze({
    epoch: MOCK_EPOCH,
    slot: MOCK_SLOT,
    participants: MOCK_PARTICIPANTS,
});

// ============================================================
// FIXED REFERENCE TIME
// ============================================================

const MOCK_REFERENCE_TIME = Date.parse('2026-09-02T09:00:00Z');

// ============================================================
// DETERMINISTIC HELPERS
// ============================================================

/**
 * Non-cryptographic deterministic hash.
 *
 * This MUST NOT be used for security, identity,
 * consensus, signatures, or cryptographic commitments.
 *
 * @param {string} input
 * @returns {number}
 */
function hashSeed(input) {
    let hash = 2166136261;

    for (let i = 0; i < input.length; i++) {
        hash ^= input.charCodeAt(i);
        hash = Math.imul(hash, 16777619);
    }

    return hash >>> 0;
}

/**
 * Generate deterministic hexadecimal data.
 *
 * This is demo data only and is not cryptographic.
 *
 * @param {string} seed
 * @param {number} length
 * @returns {string}
 */
function generateHex(seed, length) {
    if (typeof seed !== 'string' || !Number.isInteger(length) || length < 0) {
        throw new TypeError('Invalid deterministic hex parameters');
    }

    let value = hashSeed(seed);
    let result = '';

    for (let i = 0; i < length; i++) {
        value ^= value << 13;
        value ^= value >>> 17;
        value ^= value << 5;
        value >>>= 0;

        result += (value & 0x0f).toString(16);
    }

    return result;
}

/**
 * Deterministic presence percentage.
 *
 * Range: 30.00 .. 100.00
 *
 * @param {string} seed
 * @returns {number}
 */
function generatePresence(seed) {
    const value = hashSeed(seed) % 7001;
    return Math.round((30 + value / 100) * 100) / 100;
}

/**
 * Deterministic timestamp relative to fixed reference time.
 *
 * @param {number} offsetSeconds
 * @returns {string}
 */
function timestamp(offsetSeconds) {
    if (!Number.isFinite(offsetSeconds) || offsetSeconds < 0) {
        throw new TypeError('Timestamp offset must be non-negative');
    }

    return new Date(MOCK_REFERENCE_TIME - offsetSeconds * 1000).toISOString();
}

/**
 * Normalize positive epoch ID.
 *
 * @param {unknown} epochId
 * @returns {number}
 */
function normalizeEpochId(epochId) {
    if (epochId === undefined || epochId === null) {
        return state.epoch;
    }

    if (typeof epochId === 'string' && !/^\d+$/.test(epochId.trim())) {
        throw new TypeError('epochId must be a positive integer');
    }

    const id = Number(epochId);

    if (!Number.isSafeInteger(id) || id < 1) {
        throw new TypeError('epochId must be a positive integer');
    }

    return id;
}

/**
 * Normalize non-negative integer.
 *
 * @param {unknown} value
 * @param {number} fallback
 * @param {number} maximum
 * @returns {number}
 */
function normalizeInteger(value, fallback, maximum) {
    if (value === undefined || value === null) {
        return fallback;
    }

    if (typeof value === 'string' && !/^\d+$/.test(value.trim())) {
        throw new TypeError('Value must be a non-negative integer');
    }

    const number = Number(value);

    if (!Number.isSafeInteger(number) || number < 0) {
        throw new TypeError('Value must be a non-negative integer');
    }

    return Math.min(number, maximum);
}

// ============================================================
// SLOT / EPOCH CALCULATIONS
// ============================================================

/**
 * Return total number of completed slots before an epoch.
 *
 * Epoch 1 starts at slot 1.
 *
 * @param {number} epochId
 * @returns {number}
 */
function slotsBeforeEpoch(epochId) {
    return (epochId - 1) * EPOCH_SLOTS;
}

/**
 * Return emission rate for an epoch.
 *
 * Epochs:
 *   1..1461       → 20
 *   1462..2922    → 10
 *   2923..4383    → 5
 *   4384..5844    → 2.5
 *   5845+         → 1.25
 *
 * @param {number} epochId
 * @returns {number}
 */
function getEmissionPerSlot(epochId) {
    const halving = Math.floor((epochId - 1) / EPOCHS_PER_HALVING);
    return EMISSION_SCHEDULE[Math.min(halving, EMISSION_SCHEDULE.length - 1)];
}

/**
 * Calculate scheduled emission through a specific slot.
 *
 * This is intentionally separate from the protocol's actual
 * final emission accounting. It is only a deterministic mock
 * representation of the monetary schedule.
 *
 * @param {number} epochId
 * @param {number} slot
 * @returns {number}
 */
function calculateScheduledEmission(epochId, slot) {
    let total = 0;

    for (let epoch = 1; epoch < epochId; epoch++) {
        total += getEmissionPerSlot(epoch) * EPOCH_SLOTS;
    }

    total += getEmissionPerSlot(epochId) * slot;

    return Math.min(total, EMISSION_BUDGET);
}

/**
 * Current scheduled emission.
 */
const MOCK_EMITTED = calculateScheduledEmission(state.epoch, state.slot);

/**
 * Current circulating supply.
 *
 * Genesis + scheduled emission.
 *
 * Fees/L2 share are deliberately not fabricated here.
 */
const MOCK_CIRCULATING = GENESIS_SUPPLY + MOCK_EMITTED;

/**
 * Remaining monetary emission budget.
 */
const MOCK_REMAINING_EMISSION = Math.max(EMISSION_BUDGET - MOCK_EMITTED, 0);

// ============================================================
// EPOCH
// ============================================================

/**
 * Generate deterministic epoch information.
 *
 * @param {number} epochId
 * @returns {Object}
 */
function generateEpochInfo(epochId) {
    const id = normalizeEpochId(epochId);

    const slotStart = slotsBeforeEpoch(id) + 1;
    const slotEnd = id * EPOCH_SLOTS;

    const finalized = id < state.epoch;

    return {
        id,
        slotStart,
        slotEnd,

        stateRoot: '0x' + generateHex(`epoch:${id}:state`, 64),
        presenceRoot: '0x' + generateHex(`epoch:${id}:presence`, 64),
        rewardRoot: '0x' + generateHex(`epoch:${id}:reward`, 64),
        feeRoot: '0x' + generateHex(`epoch:${id}:fee`, 64),
        emissionRoot: '0x' + generateHex(`epoch:${id}:emission`, 64),
        snapshotHash: '0x' + generateHex(`epoch:${id}:snapshot`, 64),

        finalizedAt: finalized
            ? timestamp((state.epoch - id) * EPOCH_DURATION_SECONDS)
            : null,
    };
}

/**
 * Generate deterministic epoch detail.
 *
 * @param {number} epochId
 * @returns {Object}
 */
function generateEpochDetail(epochId) {
    const id = normalizeEpochId(epochId);
    const base = generateEpochInfo(id);

    return {
        ...base,

        participants: 800 + (hashSeed(`${id}:participants`) % 600),
        transactions: 100 + (hashSeed(`${id}:transactions`) % 500),
        rewards: 50 + (hashSeed(`${id}:rewards`) % 200),
    };
}

// ============================================================
// PARTICIPANTS
// ============================================================

/**
 * Generate deterministic participants.
 *
 * @param {number} limit
 * @param {number} offset
 * @returns {Object}
 */
function generateParticipants(limit = 10, offset = 0) {
    const safeLimit = normalizeInteger(limit, 10, 100);
    const safeOffset = normalizeInteger(offset, 0, state.participants);

    const end = Math.min(safeOffset + safeLimit, state.participants);
    const items = [];

    for (let index = safeOffset; index < end; index++) {
        const seed = `participant:${index}`;

        let status = 'active';

        if (index % 23 === 0) {
            status = 'inactive';
        } else if (index % 11 === 0) {
            status = 'pending';
        }

        items.push({
            address: '0x' + generateHex(`${seed}:address`, 40),
            presence: generatePresence(`${seed}:presence`),
            status,
        });
    }

    return {
        items,
        total: state.participants,
    };
}

// ============================================================
// ACTIVITY
// ============================================================

const ACTIVITY_TYPES = Object.freeze([
    'epoch',
    'snapshot',
    'transaction',
    'transaction',
    'reward',
    'presence',
    'epoch',
    'transaction',
]);

const ACTIVITY_STATUS = Object.freeze([
    'finalized',
    'finalized',
    'finalized',
    'processing',
]);

/**
 * Generate deterministic recent activity.
 *
 * @param {number} limit
 * @returns {Object}
 */
function generateActivity(limit = 10) {
    const safeLimit = normalizeInteger(limit, 10, 50);
    const items = [];

    for (let index = 0; index < safeLimit; index++) {
        const type = ACTIVITY_TYPES[index % ACTIVITY_TYPES.length];
        const epoch = Math.max(state.epoch - Math.floor(index / 2), 1);
        const slot = type === 'epoch' ? null : ((index * 347) % EPOCH_SLOTS) + 1;

        items.push({
            type,
            hash: '0x' + generateHex(`activity:${index}:${type}:${epoch}:${slot}`, 40),
            epoch,
            slot,
            status: ACTIVITY_STATUS[index % ACTIVITY_STATUS.length],
            timestamp: timestamp(index * 300),
        });
    }

    return { items };
}

// ============================================================
// TRANSACTIONS
// ============================================================

const TRANSACTION_TYPES = Object.freeze([
    'transfer',
    'reward',
    'stake',
    'unstake',
]);

const TRANSACTION_STATUSES = Object.freeze([
    'finalized',
    'finalized',
    'finalized',
    'processing',
]);

/**
 * Generate deterministic transaction detail.
 *
 * @param {string} txHash
 * @returns {Object}
 */
function generateTransactionDetail(txHash) {
    if (typeof txHash !== 'string' || !/^0x[a-f0-9]{64}$/i.test(txHash)) {
        throw new TypeError('Invalid transaction hash');
    }

    const seed = txHash.toLowerCase();

    const epoch = Math.max(state.epoch - (hashSeed(seed) % 5), 1);
    const slot = (hashSeed(`${seed}:slot`) % EPOCH_SLOTS) + 1;

    const amount = Math.round((100 + (hashSeed(`${seed}:amount`) % 900)) * 100) / 100;
    const fee = Math.round((0.01 + (hashSeed(`${seed}:fee`) % 100) / 1000) * 100) / 100;

    return {
        hash: txHash,
        epoch,
        slot,

        type: TRANSACTION_TYPES[hashSeed(seed) % TRANSACTION_TYPES.length],

        status: TRANSACTION_STATUSES[hashSeed(`${seed}:status`) % TRANSACTION_STATUSES.length],

        timestamp: timestamp(hashSeed(`${seed}:time`) % 86_400),

        from: '0x' + generateHex(`${seed}:from`, 40),
        to: '0x' + generateHex(`${seed}:to`, 40),

        amount,
        fee,
    };
}

// ============================================================
// ADDRESSES
// ============================================================

const ADDRESS_TYPES = Object.freeze([
    'participant',
    'validator',
    'observer',
]);

/**
 * Generate deterministic address detail.
 *
 * @param {string} address
 * @returns {Object}
 */
function generateAddressDetail(address) {
    if (typeof address !== 'string' || !/^0x[a-f0-9]{40}$/i.test(address)) {
        throw new TypeError('Invalid address');
    }

    const seed = address.toLowerCase();

    return {
        address,

        presence: generatePresence(`${seed}:presence`),

        status: 'active',

        firstSeen: timestamp(
            90 * 86_400 + (hashSeed(`${seed}:first`) % (30 * 86_400))
        ),

        lastSeen: timestamp(hashSeed(`${seed}:last`) % 3_600),

        balance: Math.round((1000 + (hashSeed(`${seed}:balance`) % 9000)) * 100) / 100,

        transactions: 10 + (hashSeed(`${seed}:transactions`) % 190),

        type: ADDRESS_TYPES[hashSeed(seed) % ADDRESS_TYPES.length],
    };
}

// ============================================================
// SEARCH
// ============================================================

/**
 * Generate deterministic search result.
 *
 * Search precedence:
 *   1. epoch
 *   2. transaction hash
 *   3. address
 *
 * @param {string} query
 * @returns {Object}
 */
function generateSearchResult(query) {
    if (typeof query !== 'string') {
        throw new TypeError('Search query must be a string');
    }

    const normalized = query.trim().toLowerCase();

    if (!normalized) {
        return {
            type: 'unknown',
            data: null,
        };
    }

    // Epoch
    if (/^\d+$/.test(normalized)) {
        const epochId = Number(normalized);

        if (Number.isSafeInteger(epochId) && epochId >= 1) {
            return {
                type: 'epoch',
                data: generateEpochInfo(epochId),
            };
        }
    }

    // Transaction
    if (/^0x[a-f0-9]{64}$/i.test(normalized)) {
        const epoch = Math.max(state.epoch - (hashSeed(normalized) % 5), 1);
        const slot = (hashSeed(`${normalized}:slot`) % EPOCH_SLOTS) + 1;

        return {
            type: 'transaction',

            data: {
                hash: normalized,
                epoch,
                slot,

                status: TRANSACTION_STATUSES[
                    hashSeed(`${normalized}:status`) % TRANSACTION_STATUSES.length
                ],

                timestamp: timestamp(hashSeed(`${normalized}:time`) % 86_400),
            },
        };
    }

    // Address
    if (/^0x[a-f0-9]{40}$/i.test(normalized)) {
        return {
            type: 'address',

            data: {
                address: normalized,

                presence: generatePresence(`${normalized}:presence`),

                status: 'active',

                firstSeen: timestamp(
                    90 * 86_400 + (hashSeed(`${normalized}:first`) % (30 * 86_400))
                ),

                lastSeen: timestamp(hashSeed(`${normalized}:last`) % 3_600),
            },
        };
    }

    return {
        type: 'unknown',
        data: null,
    };
}

// ============================================================
// PROVIDER
// ============================================================

export const MockProvider = {
    async getNetworkStatus() {
        return {
            epoch: state.epoch,
            slot: state.slot,

            state: state.slot > EPOCH_SLOTS * 0.8 ? 'finalizing' : 'active',

            participants: state.participants,

            supply: MOCK_CIRCULATING,

            source: 'mock',
        };
    },

    async getSupply() {
        return {
            genesis: GENESIS_SUPPLY,
            circulating: MOCK_CIRCULATING,
            maxSupply: MAX_SUPPLY,
            remainingEmission: MOCK_REMAINING_EMISSION,
            emissionPerSlot: getEmissionPerSlot(state.epoch),
        };
    },

    async getEpoch(epochId) {
        return generateEpochInfo(epochId);
    },

    async getEpochDetail(epochId) {
        return generateEpochDetail(epochId);
    },

    async getParticipants(limit, offset) {
        return generateParticipants(limit, offset);
    },

    async getRecentActivity(limit) {
        return generateActivity(limit);
    },

    async getTransactionDetail(txHash) {
        return generateTransactionDetail(txHash);
    },

    async getAddressDetail(address) {
        return generateAddressDetail(address);
    },

    async search(query) {
        return generateSearchResult(query);
    },
};

// ============================================================
// CONTRACT COMPATIBILITY
// ============================================================

/**
 * Development-time contract assertion.
 *
 * This MUST execute after every canonical provider
 * method has been declared.
 */
const contractMethods = Object.keys(AevumAPI);

for (const method of contractMethods) {
    if (typeof MockProvider[method] !== 'function') {
        throw new Error(
            `MockProvider does not implement AevumAPI.${method}()`
        );
    }
}

export default MockProvider;
