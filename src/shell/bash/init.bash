stty -ixon 2>/dev/null

ocs() {
  local session

  if (( $# )); then
    command ocs "$@"
    return
  fi

  session="$(command ocs --print)" || return

  [[ -n "$session" ]] || return

  command opencode --session "$session"
}

_ocs_search() {
  local session
  session="$(command ocs --print --query "$READLINE_LINE")"
  local exit_code="$?"

  if (( exit_code == 0 )) && [[ -n "$session" ]]; then
    printf -v READLINE_LINE 'opencode --session %q' "$session"
    READLINE_POINT="${#READLINE_LINE}"
  fi

  return "$exit_code"
}

if [[ $- == *i* ]]; then
  bind -x '"\C-s":_ocs_search'
fi
