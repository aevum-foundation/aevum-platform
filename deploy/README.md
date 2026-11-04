# deploy/

Server-side configuration templates for `aevumchain.com`.

These files are **not** installed by `aevum-deploy`. `aevum-deploy`
publishes the static site only and never touches Apache
(deployment-plan-v1.md §11). Everything here is applied by an operator,
once, by hand, with a backup and a rollback path.

| File | Installs to | Purpose |
| ---- | ----------- | ------- |
| `apache/zz-aevum-hardening.conf` | `/etc/apache2/conf-available/` | Security headers, CSP, no directory listing, caching, `ServerTokens Prod`, `/favicon.ico` |
| `apache/www-redirect.conf.example` | `/etc/apache2/sites-available/aevumchain-www.conf` | `www` → apex 301 (only if needed) |

Nothing here changes DocumentRoot, the release symlink chain
(`/var/www/html → /var/www/active → /var/www/releases/<sha>`) or the
backend routes.

---

## 0. Backup (always first)

```bash
TS=$(date +%Y%m%d-%H%M%S)
tar -czf /root/apache2-backup-$TS.tar.gz -C /etc apache2
echo /root/apache2-backup-$TS.tar.gz
```

Full rollback of everything in this document:

```bash
tar -xzf /root/apache2-backup-<TS>.tar.gz -C /etc
apachectl configtest && systemctl reload apache2
```

---

## 1. Hardening config

```bash
cd /root/aevum-platform && git pull
cp deploy/apache/zz-aevum-hardening.conf /etc/apache2/conf-available/
a2enmod headers
a2enconf zz-aevum-hardening
apachectl configtest            # must print: Syntax OK
systemctl reload apache2
```

Verify:

```bash
curl -sI https://aevumchain.com/ | grep -iE 'server|strict|x-frame|x-content|referrer|permissions|cross-origin|content-security|cache-control'
curl -s -o /dev/null -w '%{http_code}\n' https://aevumchain.com/js/          # expect 403 (was 200, directory listing)
curl -s -o /dev/null -w '%{http_code} %{redirect_url}\n' https://aevumchain.com/favicon.ico   # expect 301 → /img/aevum-logo.svg
curl -sI https://aevumchain.com/img/aevum-logo.svg | grep -i cache-control   # expect max-age=604800
```

If `/js/` still returns 200, the vhost has its own
`<Directory /var/www/html>` with `Options Indexes`. Find it and change
`Indexes` to `-Indexes` (keep `FollowSymLinks`):

```bash
grep -rn -A3 '<Directory' /etc/apache2/sites-enabled/
```

Rollback: `a2disconf zz-aevum-hardening && systemctl reload apache2`.

---

## 2. Content-Security-Policy

The config ships CSP in **Report-Only** mode. Nothing is blocked.

Enforce CSP only when **both** are deployed:

- frontend with `/js/entries/*.js` (PR #5) — `curl -sI https://aevumchain.com/js/entries/page.js` returns 200;
- backend built from the same commit — `curl -s https://aevumchain.com/growth | grep -c 'js/entries/page.js'` returns 1.

Then, in `/etc/apache2/conf-available/zz-aevum-hardening.conf`,
comment the `Content-Security-Policy-Report-Only` line and uncomment
the `Content-Security-Policy` line:

```bash
apachectl configtest && systemctl reload apache2
```

Open the site in a browser, DevTools → Console: there must be no
`Refused to …` messages.

Important: after enforcing CSP, `aevum-deploy rollback` to a release
**older than PR #5** breaks the header and footer on every page (inline
module scripts are blocked). Before such a rollback, switch CSP back to
Report-Only.

---

## 3. Phase E1: site owns `sitemap.xml` and `robots.txt`

intelligence-integration-v1.md §5.5 and §7.1: the backend must not
serve `/sitemap.xml` and `/robots.txt`. Today Apache proxies both to
the backend, so the site's own files are shadowed.

```bash
F=/etc/apache2/sites-available/aevumchain-le-ssl.conf
grep -nE 'ProxyPass(Reverse)?\s+/(sitemap\.xml|robots\.txt)' "$F"   # if empty: grep -rn 'sitemap\|robots' /etc/apache2/sites-enabled/
cp "$F" "$F.bak-$(date +%Y%m%d-%H%M%S)"
sed -i -E 's@^(\s*ProxyPass(Reverse)?\s+/(sitemap\.xml|robots\.txt)\b.*)$@# E1: removed, site owns this file\n# \1@' "$F"
grep -nE '(sitemap\.xml|robots\.txt)' "$F"
apachectl configtest && systemctl reload apache2
```

Verify (acceptance criteria of E1):

```bash
curl -s https://aevumchain.com/sitemap.xml | grep -c '<loc>'          # expect 24 with PR #4 merged (15 without)
curl -s https://aevumchain.com/sitemap.xml | grep -c 'aevumchain.com/</loc>'   # expect 1 (contains /)
curl -s https://aevumchain.com/robots.txt                              # expect the content of frontend/robots.txt
```

Rollback: copy the `.bak-…` file back, `configtest`, `reload`.

---

## 4. TLS certificates

Check expiry and auto-renewal:

```bash
certbot certificates
systemctl list-timers | grep -i certbot     # a certbot timer must be listed
certbot renew --dry-run                     # must end with "Congratulations, all simulated renewals succeeded"
```

Public CT logs (crt.sh) show the current certificates for
`aevumchain.com` expire on **2026-11-29**. If the timer is missing:

```bash
systemctl enable --now certbot.timer
```

---

## 5. `www.aevumchain.com`

Diagnose:

```bash
apachectl -S 2>&1 | grep -iE 'www|aevumchain'
curl -sI https://www.aevumchain.com/ | head -3
curl -sI http://www.aevumchain.com/  | head -3
certbot certificates | grep -A3 -i 'domains'
```

Target: both `http://www.` and `https://www.` answer
`301 → https://aevumchain.com/`.

If no certificate lists `www.aevumchain.com`, expand the existing one
(take `<CERT_NAME>` from `certbot certificates`):

```bash
certbot --apache --cert-name <CERT_NAME> -d aevumchain.com -d www.aevumchain.com
```

If no vhost answers for `www`, use the template:

```bash
cp deploy/apache/www-redirect.conf.example /etc/apache2/sites-available/aevumchain-www.conf
# edit <CERT_NAME> in the file
a2enmod rewrite ssl
a2ensite aevumchain-www
apachectl configtest && systemctl reload apache2
```

Only after `https://www.aevumchain.com/` works, `includeSubDomains`
may be added to `Strict-Transport-Security`.

Rollback: `a2dissite aevumchain-www && systemctl reload apache2`.
