#!/usr/bin/env bash
set -euo pipefail

HOST="${ANTIROT_VPS_HOST:-antirot}"
PUBLIC_URL="${ANTIROT_BACKEND_URL:-https://api.antirot.org}"
SERVICE="${ANTIROT_BACKEND_SERVICE:-antirot-backend.service}"
BACKEND_DIR="${ANTIROT_BACKEND_DIR:-/opt/antirot}"
LOCAL_BACKEND_URL="${ANTIROT_LOCAL_BACKEND_URL:-http://127.0.0.1:8787}"
LOCAL_EDGE_HOST="${ANTIROT_LOCAL_EDGE_HOST:-api.antirot.org}"

usage() {
    cat <<'EOF'
Usage: scripts/restart-backend-vps.sh <command>

Commands:
  health             Check public HTTPS health plus VPS-local backend health.
  local-health       Check only the Rust backend on 127.0.0.1:8787 from the VPS.
  public-health      Check only the public HTTPS backend URL from this machine.
  status             Show backend systemd status.
  logs               Show recent backend logs.
  restart-backend    Restart only antirot-backend.service and verify health.
  edge-check         Check nginx-to-backend routing on localhost and public HTTPS.
  reload-nginx       Reload nginx, then verify public health.
  restart-nginx      Restart nginx, then verify public health.
  rebuild-backend    Build backend on VPS, install the binary, restart, verify.
  full               Restart backend, try nginx reload if allowed, run all checks.
  rescue             Backend restart, optional nginx reload, logs tail, all checks.

Environment:
  ANTIROT_VPS_HOST          SSH host alias. Default: antirot
                            Use local to run commands directly on the VPS.
  ANTIROT_BACKEND_URL       Public backend URL. Default: https://api.antirot.org
  ANTIROT_BACKEND_SERVICE   systemd service. Default: antirot-backend.service
  ANTIROT_BACKEND_DIR       VPS checkout. Default: /opt/antirot
  ANTIROT_LOCAL_BACKEND_URL VPS-local backend URL. Default: http://127.0.0.1:8787
  ANTIROT_LOCAL_EDGE_HOST   HTTPS vhost to test on 127.0.0.1. Default: api.antirot.org

Notes:
  - Backend restart/status use exact /usr/bin/systemctl paths for sudoers.
  - nginx commands require separate sudoers access or an interactive sudo password.
  - If local health works but public health fails, the Rust backend is probably
    not the problem. Check nginx, DNS, firewall, or host-network reachability.
EOF
}

ssh_vps() {
    if [[ "$HOST" == "local" || "$HOST" == "localhost" ]]; then
        bash -lc "$*"
        return
    fi

    ssh "$HOST" "$@"
}

step() {
    printf '\n==> %s\n' "$1"
}

run_check() {
    local label="$1"
    shift

    step "$label"
    if "$@"; then
        printf 'ok: %s\n' "$label"
        return 0
    fi

    printf 'failed: %s\n' "$label" >&2
    return 1
}

public_health() {
    curl -fsS --max-time 12 "$PUBLIC_URL/v1/health" || return $?
    printf '\n'
}

local_health() {
    ssh_vps "curl -fsS --max-time 8 '$LOCAL_BACKEND_URL/v1/health' || exit \$?; printf '\n'"
}

status_backend() {
    ssh_vps "sudo -n /usr/bin/systemctl status $SERVICE --no-pager --full"
}

logs_backend() {
    ssh_vps "sudo -n /usr/bin/journalctl -u $SERVICE -n 120 --no-pager"
}

restart_backend() {
    ssh_vps "sudo -n /usr/bin/systemctl restart $SERVICE"
    run_check "VPS-local backend health" local_health
    run_check "public backend health" public_health
}

edge_check() {
    ssh_vps "curl -k -fsS --max-time 8 --resolve '$LOCAL_EDGE_HOST:443:127.0.0.1' 'https://$LOCAL_EDGE_HOST/v1/health' || exit \$?; printf '\n'"
}

listener_check() {
    ssh_vps "ss -ltnp | grep -E '(:443|:80|:8787)\\b' || true"
}

health_all() {
    local failed=0
    run_check "public backend health from local machine" public_health || failed=1
    run_check "VPS-local Rust backend health" local_health || failed=1
    return "$failed"
}

edge_check_all() {
    local failed=0
    run_check "VPS-local nginx HTTPS vhost health" edge_check || failed=1
    run_check "public backend health from local machine" public_health || failed=1
    run_check "VPS listening ports" listener_check || failed=1
    return "$failed"
}

reload_nginx() {
    ssh_vps 'sudo -n /usr/bin/systemctl reload nginx'
    run_check "public backend health" public_health
}

restart_nginx() {
    ssh_vps 'sudo -n /usr/bin/systemctl restart nginx'
    run_check "public backend health" public_health
}

rebuild_backend() {
    ssh_vps "cd '$BACKEND_DIR' && git pull --ff-only origin main && /home/antirot/.cargo/bin/cargo build --manifest-path apps/backend/Cargo.toml --release && install -m 755 apps/backend/target/release/antirot-backend apps/backend/antirot-backend && sudo -n /usr/bin/systemctl restart '$SERVICE'"
    run_check "VPS-local backend health" local_health
    run_check "public backend health" public_health
}

full_restart() {
    local failed=0

    run_check "backend service status before restart" status_backend || failed=1
    run_check "backend service restart" restart_backend || failed=1

    if run_check "nginx reload" reload_nginx; then
        :
    else
        failed=1
        printf 'nginx reload failed. If this says sudo password required, add sudoers for nginx reload/restart or run it manually.\n' >&2
    fi

    run_check "edge checks" edge_check_all || failed=1
    return "$failed"
}

rescue() {
    local failed=0

    run_check "backend service restart" restart_backend || failed=1
    run_check "nginx reload" reload_nginx || {
        failed=1
        printf 'nginx reload skipped/failed. Current sudoers may only allow backend restart/status.\n' >&2
    }
    run_check "backend logs tail" logs_backend || failed=1
    run_check "edge checks" edge_check_all || failed=1
    return "$failed"
}

command="${1:-}"
case "$command" in
    health)
        health_all
        ;;
    local-health)
        local_health
        ;;
    public-health)
        public_health
        ;;
    status)
        status_backend
        ;;
    logs)
        logs_backend
        ;;
    restart-backend)
        restart_backend
        ;;
    edge-check)
        edge_check_all
        ;;
    reload-nginx)
        reload_nginx
        ;;
    restart-nginx)
        restart_nginx
        ;;
    rebuild-backend)
        rebuild_backend
        ;;
    full)
        full_restart
        ;;
    rescue)
        rescue
        ;;
    -h|--help|help|"")
        usage
        ;;
    *)
        printf 'Unknown command: %s\n\n' "$command" >&2
        usage >&2
        exit 2
        ;;
esac
