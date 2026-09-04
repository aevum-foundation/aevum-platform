/**
 * AEVUM WEB PLATFORM
 * CANONICAL API CONTRACT v2
 *
 * Frontend boundary:
 *
 *   UI
 *    ↓
 *   Store
 *    ↓
 *   AevumAPI
 *    ↓
 *   Provider
 *      ├── NetworkProvider
 *      └── MockProvider
 *
 * The contract is provider-independent.
 *
 * IMPORTANT:
 * - Slot/epoch native.
 * - No block-centric semantics.
 * - No provider implementation here.
 * - No network access here.
 * - No mutable shared state here.
 */

const NOT_CONFIGURED =
    'AevumAPI provider not configured';

/**
 * Canonical frontend API surface.
 *
 * Providers are expected to implement every method below.
 */
export const AevumAPI = Object.freeze({

    // ==========================================================
    // NETWORK
    // ==========================================================

    /**
     * Get current network status.
     *
     * @returns {Promise<NetworkStatus>}
     */
    getNetworkStatus: async () => {
        throw new Error(NOT_CONFIGURED);
    },

    /**
     * Get current monetary state.
     *
     * @returns {Promise<SupplyInfo>}
     */
    getSupply: async () => {
        throw new Error(NOT_CONFIGURED);
    },

    // ==========================================================
    // EPOCH / FINALITY
    // ==========================================================

    /**
     * Get canonical epoch information.
     *
     * @param {number} [epochId]
     * @returns {Promise<EpochInfo>}
     */
    getEpoch: async (epochId) => {
        throw new Error(NOT_CONFIGURED);
    },

    /**
     * Get detailed epoch information.
     *
     * @param {number} epochId
     * @returns {Promise<EpochDetail>}
     */
    getEpochDetail: async (epochId) => {
        throw new Error(NOT_CONFIGURED);
    },

    // ==========================================================
    // PARTICIPANTS
    // ==========================================================

    /**
     * Get equal-share protocol participants.
     *
     * No stake-weighted influence is implied.
     *
     * @param {number} [limit]
     * @param {number} [offset]
     * @returns {Promise<ParticipantsList>}
     */
    getParticipants: async (limit, offset) => {
        throw new Error(NOT_CONFIGURED);
    },

    // ==========================================================
    // ACTIVITY
    // ==========================================================

    /**
     * Get recent network activity.
     *
     * Activity is slot/epoch native.
     *
     * @param {number} [limit]
     * @returns {Promise<ActivityList>}
     */
    getRecentActivity: async (limit) => {
        throw new Error(NOT_CONFIGURED);
    },

    // ==========================================================
    // TRANSACTIONS
    // ==========================================================

    /**
     * Get detailed transaction information.
     *
     * @param {string} txHash
     * @returns {Promise<TransactionDetail>}
     */
    getTransactionDetail: async (txHash) => {
        throw new Error(NOT_CONFIGURED);
    },

    // ==========================================================
    // ADDRESSES
    // ==========================================================

    /**
     * Get detailed address information.
     *
     * @param {string} address
     * @returns {Promise<AddressDetail>}
     */
    getAddressDetail: async (address) => {
        throw new Error(NOT_CONFIGURED);
    },

    // ==========================================================
    // SEARCH
    // ==========================================================

    /**
     * Search the Aevum network.
     *
     * @param {string} query
     * @returns {Promise<SearchResult>}
     */
    search: async (query) => {
        throw new Error(NOT_CONFIGURED);
    },
});


// ============================================================
// TYPE DEFINITIONS
// ============================================================

/**
 * @typedef {Object} NetworkStatus
 *
 * @property {number} epoch
 * @property {number} slot
 * @property {NetworkState} state
 * @property {number} participants
 * @property {number} supply
 * @property {NetworkSource} source
 */

/**
 * @typedef {'active'|'finalizing'|'degraded'|'offline'} NetworkState
 */

/**
 * @typedef {'network'|'mock'} NetworkSource
 */


/**
 * @typedef {Object} EpochInfo
 *
 * @property {number} id
 * @property {number} slotStart
 * @property {number} slotEnd
 * @property {string} stateRoot
 * @property {string} presenceRoot
 * @property {string} rewardRoot
 * @property {string} feeRoot
 * @property {string} emissionRoot
 * @property {string} snapshotHash
 * @property {string|null} finalizedAt
 */


/**
 * @typedef {Object} EpochDetail
 *
 * @property {number} id
 * @property {number} slotStart
 * @property {number} slotEnd
 * @property {string} stateRoot
 * @property {string} presenceRoot
 * @property {string} rewardRoot
 * @property {string} feeRoot
 * @property {string} emissionRoot
 * @property {string} snapshotHash
 * @property {string|null} finalizedAt
 * @property {number} participants
 * @property {number} transactions
 * @property {number} rewards
 */


/**
 * @typedef {Object} SupplyInfo
 *
 * @property {number} genesis
 * @property {number} circulating
 * @property {number} maxSupply
 * @property {number} remainingEmission
 * @property {number} emissionPerSlot
 */


/**
 * @typedef {Object} ParticipantsList
 *
 * @property {Participant[]} items
 * @property {number} total
 */


/**
 * @typedef {Object} Participant
 *
 * @property {string} address
 * @property {number} presence
 * @property {ParticipantStatus} status
 */


/**
 * @typedef {'active'|'inactive'|'pending'} ParticipantStatus
 */


/**
 * @typedef {Object} ActivityList
 *
 * @property {Activity[]} items
 */


/**
 * @typedef {Object} Activity
 *
 * @property {ActivityType} type
 * @property {string} hash
 * @property {number} epoch
 * @property {number|null} slot
 * @property {ActivityStatus} status
 * @property {string} timestamp
 */


/**
 * @typedef {
 *   'epoch'|
 *   'snapshot'|
 *   'transaction'|
 *   'reward'|
 *   'presence'
 * } ActivityType
 */


/**
 * @typedef {'finalized'|'processing'} ActivityStatus
 */


/**
 * @typedef {Object} TransactionDetail
 *
 * @property {string} hash
 * @property {number} epoch
 * @property {number} slot
 * @property {TransactionType} type
 * @property {ActivityStatus} status
 * @property {string} timestamp
 * @property {string} from
 * @property {string} to
 * @property {number} amount
 * @property {number} fee
 */


/**
 * @typedef {
 *   'transfer'|
 *   'reward'|
 *   'stake'|
 *   'unstake'
 * } TransactionType
 */


/**
 * @typedef {Object} AddressDetail
 *
 * @property {string} address
 * @property {number} presence
 * @property {ParticipantStatus} status
 * @property {string} firstSeen
 * @property {string} lastSeen
 * @property {number} balance
 * @property {number} transactions
 * @property {AddressType} type
 */


/**
 * @typedef {
 *   'participant'|
 *   'validator'|
 *   'observer'
 * } AddressType
 */


/**
 * @typedef {Object} SearchResult
 *
 * @property {SearchResultType} type
 * @property {unknown|null} data
 */


/**
 * @typedef {
 *   'epoch'|
 *   'transaction'|
 *   'address'|
 *   'unknown'
 * } SearchResultType
 */
