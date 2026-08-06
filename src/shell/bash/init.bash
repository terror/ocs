ocs() {
  local session

  if (( $# )); then
    command ocs "$@"
    return
  fi

  session="$(command ocs --print)" || return

  [[ -n "$session" ]] || return

  command ocs --session "$session"
}

_ocs_search() {
  local session
  session="$(command ocs --print --query "$READLINE_LINE")"
  local exit_code="$?"

  if (( exit_code == 0 )) && [[ -n "$session" ]]; then
    READLINE_LINE=""
    READLINE_POINT=0
    command ocs --session "$session"
    exit_code="$?"
  fi

  return "$exit_code"
}

if [[ $- == *i* ]]; then
  bind -x '"\C-xs":_ocs_search'
fi
