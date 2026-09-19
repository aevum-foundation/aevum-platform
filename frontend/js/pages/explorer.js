/**
 * Aevum Explorer Page Module
 *
 * Page-specific logic for explorer.html.
 *
 * Contract:
 *   initExplorerPage() — async, idempotent.
 *
 * Responsibilities:
 *   - Store lifecycle (data loading, polling).
 *   - Search, detail rendering, URL routing.
 *
 * Does not:
 *   - Load header or footer (js/components.js).
 *   - Handle mobile drawer (js/navigation.js).
 */

import { Store } from '/js/api/store.js';

const LOG_PREFIX = '[Aevum Explorer]';

let initialized = false;

function validateEpochId(value) {
    if (typeof value !== 'string' || !/^\d+$/.test(value.trim())) {
        return null;
    }

    const number = Number(value.trim());

    return Number.isSafeInteger(number) && number >= 1
        ? number
        : null;
}

function validateTxHash(value) {
    if (
        typeof value !== 'string' ||
        !/^0x[a-f0-9]{64}$/i.test(value.trim())
    ) {
        return null;
    }

    return value.trim().toLowerCase();
}

function validateAddress(value) {
    if (
        typeof value !== 'string' ||
        !/^0x[a-f0-9]{40}$/i.test(value.trim())
    ) {
        return null;
    }

    return value.trim().toLowerCase();
}

function createDetailItem(label, value, className = '') {
    const item = document.createElement('div');
    item.className = 'detail-item';

    const labelEl = document.createElement('span');
    labelEl.className = 'label';
    labelEl.textContent = label;

    const valueEl = document.createElement('span');
    valueEl.className = `value${className ? ` ${className}` : ''}`;
    valueEl.textContent = value == null ? '—' : String(value);

    item.append(labelEl, valueEl);

    return item;
}

function createDetailGrid(entries) {
    const grid = document.createElement('div');
    grid.className = 'detail-grid';

    for (const [label, value, className] of entries) {
        grid.appendChild(
            createDetailItem(label, value, className)
        );
    }

    return grid;
}

function safeDate(value, fallback = '—') {
    if (!value) return fallback;

    const date = new Date(value);

    return Number.isNaN(date.getTime())
        ? fallback
        : date.toLocaleString();
}

function safeTime(value, fallback = '—') {
    if (!value) return fallback;

    const date = new Date(value);

    return Number.isNaN(date.getTime())
        ? fallback
        : date.toLocaleTimeString();
}

function renderDetailEpoch(data) {
    const title = document.createElement('h2');
    title.id = 'detail-title';
    title.textContent = `Epoch #${data.id}`;

    const grid = createDetailGrid([
        ['Slot Start', data.slotStart],
        ['Slot End', data.slotEnd],
        ['State Root', data.stateRoot],
        ['Presence Root', data.presenceRoot],
        ['Reward Root', data.rewardRoot],
        ['Fee Root', data.feeRoot],
        ['Emission Root', data.emissionRoot],
        ['Snapshot Hash', data.snapshotHash],
        ['Participants', data.participants ?? '—', 'gold'],
        ['Transactions', data.transactions ?? '—', 'gold'],
        ['Rewards', data.rewards ?? '—', 'gold'],
        ['Finalized', safeDate(data.finalizedAt, 'Pending'), 'cyan'],
    ]);

    const container = document.createElement('div');
    container.append(title, grid);

    return container;
}

function renderDetailTransaction(data) {
    const title = document.createElement('h2');
    title.id = 'detail-title';
    title.textContent = 'Transaction';

    const grid = createDetailGrid([
        ['Hash', data.hash],
        ['Epoch', data.epoch == null ? '—' : `#${data.epoch}`, 'gold'],
        ['Slot', data.slot],
        ['Type', data.type, 'cyan'],
        ['From', data.from],
        ['To', data.to],
        ['Amount', data.amount == null ? '—' : `${data.amount} AEV`, 'gold'],
        ['Fee', data.fee == null ? '—' : `${data.fee} AEV`],
        ['Status', data.status, 'cyan'],
        ['Timestamp', safeDate(data.timestamp)],
    ]);

    const container = document.createElement('div');
    container.append(title, grid);

    return container;
}

function renderDetailAddress(data) {
    const title = document.createElement('h2');
    title.id = 'detail-title';
    title.textContent = 'Address';

    const grid = createDetailGrid([
        ['Address', data.address],
        ['Type', data.type, 'cyan'],
        ['Status', data.status, 'gold'],
        ['Presence', data.presence == null ? '—' : `${data.presence}%`],
        ['Balance', data.balance == null ? '—' : `${data.balance} AEV`, 'gold'],
        ['Transactions', data.transactions],
        ['First Seen', safeDate(data.firstSeen)],
        ['Last Seen', safeDate(data.lastSeen)],
    ]);

    const container = document.createElement('div');
    container.append(title, grid);

    return container;
}

function renderDetail(type, data) {
    switch (type) {
        case 'epoch':
            return renderDetailEpoch(data);

        case 'transaction':
            return renderDetailTransaction(data);

        case 'address':
            return renderDetailAddress(data);

        default: {
            const p = document.createElement('p');
            p.textContent = 'Unknown detail type';
            return p;
        }
    }
}

export async function initExplorerPage() {
    if (initialized) {
        return;
    }

    const statEpoch = document.getElementById('stat-epoch');
    const statSupply = document.getElementById('stat-supply');
    const statParticipants = document.getElementById('stat-participants');
    const statStatus = document.getElementById('stat-status');
    const liveStatus = document.getElementById('network-live-status');

    const activityList = document.getElementById('activity-list');

    const searchForm = document.getElementById('explorer-search');
    const searchInput = document.getElementById('searchInput');
    const searchButton = document.getElementById('searchButton');
    const searchStatus = document.getElementById('search-status');

    const detailView = document.getElementById('detail-view');
    const detailContent = document.getElementById('detail-content');
    const detailBack = document.getElementById('detail-back');

    if (!statEpoch || !activityList || !searchForm || !detailView) {
        console.warn(`${LOG_PREFIX} required elements not found`);
        return;
    }

    initialized = true;

    function showDetail(type, data) {
        detailContent.replaceChildren(
            renderDetail(type, data)
        );

        detailView.classList.add('active');
        detailView.setAttribute('aria-hidden', 'false');

        window.scrollTo({
            top: 0,
            behavior: 'smooth',
        });
    }

    function hideDetail({ scroll = false } = {}) {
        detailView.classList.remove('active');
        detailView.setAttribute('aria-hidden', 'true');
        detailContent.replaceChildren();

        if (scroll) {
            window.scrollTo({
                top: 0,
                behavior: 'smooth',
            });
        }
    }

    function renderStats(network, supply) {
        if (network) {
            statEpoch.textContent = network.epoch ?? '—';

            statParticipants.textContent =
                network.participants == null
                    ? '—'
                    : Number(network.participants).toLocaleString();

            statEpoch.classList.remove('loading');
            statParticipants.classList.remove('loading');

            const state = network.state ?? 'unknown';
            const source = network.source ?? 'unknown';

            statStatus.className =
                'explorer-stat-value explorer-stat-status';

            if (source === 'mock') {
                statStatus.classList.add('demo');
            } else if (state === 'active') {
                statStatus.classList.add('live');
            } else {
                statStatus.classList.add('offline');
            }

            const statusLabel =
                state === 'active'
                    ? 'Live'
                    : state === 'finalizing'
                        ? 'Finalizing'
                        : state;

            statStatus.replaceChildren();

            const statusText = document.createTextNode(statusLabel);

            const badge = document.createElement('span');
            badge.className =
                `source-badge ${
                    source === 'mock'
                        ? 'mock'
                        : source === 'network'
                            ? 'network'
                            : ''
                }`;

            badge.textContent =
                source === 'mock'
                    ? 'DEMO'
                    : source === 'network'
                        ? 'LIVE'
                        : source.toUpperCase();

            statStatus.append(statusText, badge);

            if (source === 'mock') {
                liveStatus.textContent = 'Demo Mode';
                liveStatus.style.color = 'var(--color-accent-gold)';
            } else if (source === 'network') {
                liveStatus.textContent = 'Live';
                liveStatus.style.color = 'var(--color-accent-cyan)';
            } else {
                liveStatus.textContent = 'Unavailable';
                liveStatus.style.color = '';
            }
        }

        if (supply) {
            const circulating = Number(supply.circulating);

            statSupply.textContent =
                Number.isFinite(circulating)
                    ? `${(circulating / 1_000_000).toFixed(1)}M`
                    : '—';

            statSupply.classList.remove('loading');
        }
    }

    function getActivityRoute(item) {
        switch (item?.type) {
            case 'epoch':
            case 'snapshot':
                return {
                    type: 'epoch',
                    id: validateEpochId(String(item.epoch ?? '')),
                };

            case 'transaction':
                return {
                    type: 'transaction',
                    id: validateTxHash(item.hash),
                };

            default:
                return {
                    type: null,
                    id: null,
                };
        }
    }

    async function openActivity(item) {
        const route = getActivityRoute(item);

        if (!route.type || !route.id) {
            return;
        }

        try {
            if (route.type === 'epoch') {
                const detail = await Store.getEpochDetail(route.id);

                window.history.pushState({}, '', `?epoch=${route.id}`);

                showDetail('epoch', detail);
                return;
            }

            if (route.type === 'transaction') {
                const detail = await Store.getTransactionDetail(route.id);

                window.history.pushState({}, '', `?tx=${route.id}`);

                showDetail('transaction', detail);
            }
        } catch (error) {
            console.error(
                `${LOG_PREFIX} Failed to load activity detail:`,
                error
            );

            setSearchStatus(
                'Failed to load the selected network record.'
            );
        }
    }

    function renderActivity(activity) {
        activityList.replaceChildren();

        if (
            !activity ||
            !Array.isArray(activity.items) ||
            activity.items.length === 0
        ) {
            const li = document.createElement('li');

            li.className = 'recent-row';
            li.style.justifyContent = 'center';
            li.style.color = 'var(--color-text-muted)';
            li.style.padding = '24px';

            li.textContent = 'No recent activity';

            activityList.appendChild(li);
            return;
        }

        for (const item of activity.items) {
            const li = document.createElement('li');

            const route = getActivityRoute(item);
            const clickable = Boolean(route.type && route.id);

            li.className = 'recent-row';
            li.dataset.type = item.type ?? 'unknown';
            li.dataset.id = String(item.epoch ?? '');

            if (clickable) {
                li.dataset.clickable = 'true';
                li.tabIndex = 0;
                li.setAttribute('role', 'button');

                li.addEventListener('click', () => openActivity(item));

                li.addEventListener('keydown', (event) => {
                    if (event.key === 'Enter' || event.key === ' ') {
                        event.preventDefault();
                        openActivity(item);
                    }
                });
            }

            const hash = document.createElement('span');
            hash.className = 'recent-hash';
            hash.textContent = item.hash ?? '—';

            const epoch = document.createElement('span');
            epoch.className = 'recent-epoch';

            if (item.epoch != null) {
                epoch.textContent = `Epoch #${item.epoch}`;
            } else {
                epoch.textContent = item.type ?? 'Activity';
            }

            const badge = document.createElement('span');
            badge.className = 'explorer-badge';
            badge.textContent = item.status ?? 'unknown';

            if (item.status === 'finalized') {
                badge.classList.add('explorer-badge-finalized');
            } else if (item.status === 'processing') {
                badge.classList.add('explorer-badge-processing');
            } else {
                badge.classList.add('explorer-badge-loading');
            }

            const time = document.createElement('time');
            time.className = 'recent-time';
            time.textContent = safeTime(item.timestamp);

            if (item.timestamp) {
                time.dateTime = new Date(item.timestamp).toISOString();
            }

            li.append(hash, epoch, badge, time);
            activityList.appendChild(li);
        }
    }

    function showActivityMessage(message) {
        activityList.replaceChildren();

        const li = document.createElement('li');

        li.className = 'recent-row';
        li.style.justifyContent = 'center';
        li.style.color = 'var(--color-status-offline)';
        li.style.padding = '24px';

        li.textContent = message;

        activityList.appendChild(li);
    }

    async function loadData() {
        try {
            const [network, supply, activity] = await Promise.all([
                Store.getNetworkStatus(),
                Store.getSupply(),
                Store.getRecentActivity(10),
            ]);

            renderStats(network, supply);
            renderActivity(activity);
        } catch (error) {
            console.error(`${LOG_PREFIX} Failed to load data:`, error);

            showActivityMessage(
                'Failed to load network data. Please try again later.'
            );
        }
    }

    async function loadDetailFromUrl() {
        const params = new URLSearchParams(window.location.search);

        const epochParam = params.get('epoch');

        if (epochParam !== null) {
            const id = validateEpochId(epochParam);

            if (id !== null) {
                try {
                    const detail = await Store.getEpochDetail(id);

                    showDetail('epoch', detail);
                    return;
                } catch (error) {
                    console.error(
                        `${LOG_PREFIX} Failed to load epoch:`,
                        error
                    );
                }
            }
        }

        const txParam = params.get('tx');

        if (txParam !== null) {
            const hash = validateTxHash(txParam);

            if (hash !== null) {
                try {
                    const detail = await Store.getTransactionDetail(hash);

                    showDetail('transaction', detail);
                    return;
                } catch (error) {
                    console.error(
                        `${LOG_PREFIX} Failed to load transaction:`,
                        error
                    );
                }
            }
        }

        const addressParam = params.get('address');

        if (addressParam !== null) {
            const address = validateAddress(addressParam);

            if (address !== null) {
                try {
                    const detail = await Store.getAddressDetail(address);

                    showDetail('address', detail);
                    return;
                } catch (error) {
                    console.error(
                        `${LOG_PREFIX} Failed to load address:`,
                        error
                    );
                }
            }
        }

        hideDetail();
    }

    function setSearchStatus(message) {
        searchStatus.textContent = message ?? '';
    }

    function setSearchLoading(loading) {
        searchButton.disabled = loading;
        searchInput.disabled = loading;

        searchButton.textContent = loading ? 'Searching…' : 'Search';
    }

    searchForm.addEventListener('submit', async (event) => {
        event.preventDefault();

        const query = searchInput.value.trim();

        if (query.length < 2) {
            setSearchStatus('Enter at least 2 characters.');
            return;
        }

        setSearchStatus('');
        setSearchLoading(true);

        try {
            const result = await Store.search(query);

            if (result && result.type === 'epoch' && result.data) {
                const id = validateEpochId(String(result.data.id));

                if (id !== null) {
                    window.history.pushState({}, '', `?epoch=${id}`);

                    await loadDetailFromUrl();
                    return;
                }
            }

            if (result && result.type === 'transaction' && result.data) {
                const hash = validateTxHash(result.data.hash);

                if (hash !== null) {
                    window.history.pushState({}, '', `?tx=${hash}`);

                    await loadDetailFromUrl();
                    return;
                }
            }

            if (result && result.type === 'address' && result.data) {
                const address = validateAddress(result.data.address);

                if (address !== null) {
                    window.history.pushState({}, '', `?address=${address}`);

                    await loadDetailFromUrl();
                    return;
                }
            }

            setSearchStatus(`No results found for "${query}".`);
        } catch (error) {
            console.error(`${LOG_PREFIX} Search failed:`, error);

            setSearchStatus('Search failed. Please try again.');
        } finally {
            setSearchLoading(false);
        }
    });

    detailBack.addEventListener('click', () => {
        window.history.pushState({}, '', '/explorer.html');

        hideDetail({
            scroll: true,
        });
    });

    window.addEventListener('popstate', () => {
        loadDetailFromUrl().catch((error) => {
            console.error(`${LOG_PREFIX} Navigation failed:`, error);
        });
    });

    await loadData();
    await loadDetailFromUrl();

    Store.startPolling(30_000);

    console.log(`${LOG_PREFIX} Data layer initialized with Store`);
}
