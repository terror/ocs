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
  emulate -L zsh

  zle -I

  local session
  session="$(command ocs --print --query "$BUFFER")"
  local exit_code="$?"

  if (( exit_code == 0 )) && [[ -n "$session" ]]; then
    BUFFER="opencode --session ${(q)session}"
    CURSOR="${#BUFFER}"
  fi

  zle reset-prompt

  return "$exit_code"
}

zle -N ocs-search _ocs_search
bindkey '^Xs' ocs-search
