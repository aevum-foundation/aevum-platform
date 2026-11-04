# deploy/systemd/

`aevum-platform-api.service` — systemd unit for the backend
(Phase E1, intelligence-integration-v1.md §5.9).

The unit follows §5.9 exactly: `User=root` for v1, `Restart=on-failure`,
`AEVUM_GROWTH_DB` and `AEVUM_INTELLIGENCE_REFRESH_INTERVAL_SECS` in the
environment, no `nohup`. Moving to a dedicated `aevum` user is deferred
(§9.7) and is not part of this unit.

Sandboxing (`ProtectSystem=full`, `NoNewPrivileges`, empty
`CapabilityBoundingSet`, …) is compatible with root and with data under
`/root`. `systemd-analyze security aevum-platform-api` → 3.6 (OK).

Paths assumed by the unit:

| What | Path |
| ---- | ---- |
| Source checkout | `/root/aevum-platform` |
| Binary | `/root/aevum-platform/backend/target/release/aevum-platform-api` |
| Working directory | `/root/aevum-platform/backend` |
| Growth DB | `/root/aevum-platform/backend/data/growth` |
| Listen | `127.0.0.1:8080` (Apache ProxyPass target) |
| Local overrides | `/etc/aevum/platform-api.env` (optional, not in git) |

---

## 1. Inspect the running backend (before anything else)

AevumDB holds a single-writer lock on its directory. Two processes on
the same DB cannot run at once, so the current manual process must be
stopped before the unit starts. First record how it runs today:

```bash
PID=$(pgrep -f aevum-platform-api | head -1); echo "PID=$PID"
readlink /proc/$PID/exe          # binary
readlink /proc/$PID/cwd          # working directory
tr '\0' '\n' < /proc/$PID/environ | grep -E '^(AEVUM_|RUST_LOG)'
ss -ltnp | grep 8080
```

If the DB path or any `AEVUM_*` value differs from the table above,
put the real values into `/etc/aevum/platform-api.env`, for example:

```bash
mkdir -p /etc/aevum
cat > /etc/aevum/platform-api.env <<'EOF'
AEVUM_GROWTH_DB=/root/aevum-platform/backend/data/growth
EOF
chmod 600 /etc/aevum/platform-api.env
```

Values in that file override the defaults in the unit.

---

## 2. Build

```bash
cd /root/aevum-platform && git pull
cd backend && cargo build --release
ls -l target/release/aevum-platform-api
```

---

## 3. Install and switch over

```bash
cp /root/aevum-platform/deploy/systemd/aevum-platform-api.service /etc/systemd/system/
systemd-analyze verify /etc/systemd/system/aevum-platform-api.service
systemctl daemon-reload

kill "$PID"                       # stop the manual process (SIGTERM, graceful)
sleep 2; pgrep -f aevum-platform-api || echo stopped

systemctl enable --now aevum-platform-api
systemctl status aevum-platform-api --no-pager
```

Downtime is the few seconds between `kill` and `enable --now`.

---

## 4. Verify (E1 acceptance criteria)

```bash
systemctl is-active aevum-platform-api                                      # active
systemctl is-enabled aevum-platform-api                                     # enabled
curl -s  -o /dev/null -w '%{http_code}\n' https://aevumchain.com/api/growth/health   # 200
curl -sI -o /dev/null -w '%{http_code}\n' https://aevumchain.com/api/growth/health   # 200 (HEAD)
curl -sI -o /dev/null -w '%{http_code}\n' https://aevumchain.com/api/growth/topics   # 200 (HEAD)
curl -sI -o /dev/null -w '%{http_code}\n' https://aevumchain.com/growth              # 200 (HEAD, was 404)
journalctl -u aevum-platform-api -n 20 --no-pager
```

Reboot test (in a maintenance window):

```bash
reboot
# after boot:
systemctl is-active aevum-platform-api && curl -s -o /dev/null -w '%{http_code}\n' https://aevumchain.com/api/growth/health
```

---

## 5. Everyday operations

```bash
# new backend release
cd /root/aevum-platform && git pull && cd backend && cargo build --release
systemctl restart aevum-platform-api
curl -s -o /dev/null -w '%{http_code}\n' https://aevumchain.com/api/growth/health

# logs
journalctl -u aevum-platform-api -f
```

---

## 6. Rollback

Back to the previous binary:

```bash
cd /root/aevum-platform && git checkout <previous-sha> && cd backend && cargo build --release
systemctl restart aevum-platform-api
```

Back to manual startup (removes the unit entirely):

```bash
systemctl disable --now aevum-platform-api
rm /etc/systemd/system/aevum-platform-api.service
systemctl daemon-reload
# start the backend the way it was started before (see step 1 notes)
```
