version = 1
provider = "earl"
categories = ["meta", "bash"]

# ---------------------------------------------------------------------------
# Read operations
# ---------------------------------------------------------------------------

command "list_commands" {
  title       = "List commands"
  summary     = "List all available template commands"
  description = "Lists every command registered in Earl's local and global templates. Optionally filter by category or read/write mode."
  categories  = ["templates"]

  annotations {
    mode = "read"
  }

  param "category" {
    type        = "string"
    required    = false
    description = "Filter by category (e.g. 'system', 'scm')"
  }

  param "mode" {
    type        = "string"
    required    = false
    description = "Filter by mode: 'read' or 'write'"
  }

  operation {
    protocol = "bash"

    bash {
      script = <<-EOT
        earl templates list --json
        {%- if args.category %} --category {{ args.category }}{% endif %}
        {%- if args.mode %} --mode {{ args.mode }}{% endif %}
      EOT
      sandbox {
        network = false
      }
    }
  }

  result {
    decode = "json"
  }
}

command "search_commands" {
  title       = "Search commands"
  summary     = "Semantic search over template commands"
  description = "Searches Earl's template index using a natural-language query and returns the top matching commands."
  categories  = ["templates"]

  annotations {
    mode = "read"
  }

  param "query" {
    type        = "string"
    required    = true
    description = "Natural-language search query"
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 10
    description = "Maximum number of results"
  }

  operation {
    protocol = "bash"

    bash {
      script = "earl templates search --json --limit {{ args.limit }} '{{ args.query }}'"
      sandbox {
        network = false
      }
    }
  }

  result {
    decode = "json"
  }
}

command "validate_templates" {
  title       = "Validate templates"
  summary     = "Validate all template files"
  description = "Parses and validates every template file in Earl's local and global template directories, reporting any errors."
  categories  = ["templates"]

  annotations {
    mode = "read"
  }

  operation {
    protocol = "bash"

    bash {
      script = "earl templates validate --json"
      sandbox {
        network = false
      }
    }
  }

  result {
    decode = "json"
  }
}

command "doctor" {
  title       = "Run doctor"
  summary     = "Diagnose configuration and setup issues"
  description = "Runs Earl's built-in diagnostic checks and reports the health of configuration, templates, secrets, and network settings."
  categories  = ["diagnostics"]

  annotations {
    mode = "read"
  }

  operation {
    protocol = "bash"

    bash {
      script = "earl doctor --json"
      sandbox {
        network = false
      }
    }
  }

  result {
    decode = "json"
  }
}

command "list_secrets" {
  title       = "List secrets"
  summary     = "List all known secret keys"
  description = "Lists the keys of every secret stored in Earl's secure keychain."
  categories  = ["secrets"]

  annotations {
    mode = "read"
  }

  operation {
    protocol = "bash"

    bash {
      script = "earl secrets list"
      sandbox {
        network = false
      }
    }
  }
}

command "get_secret" {
  title       = "Get secret metadata"
  summary     = "Show metadata for a secret key"
  description = "Displays metadata (but not the value) for a secret stored in Earl's keychain."
  categories  = ["secrets"]

  annotations {
    mode = "read"
  }

  param "key" {
    type        = "string"
    required    = true
    description = "Secret key name (e.g. 'github.token')"
  }

  operation {
    protocol = "bash"

    bash {
      script = "earl secrets get '{{ args.key }}'"
      sandbox {
        network = false
      }
    }
  }
}

command "auth_status" {
  title       = "Auth status"
  summary     = "Show OAuth token status"
  description = "Shows the current authentication status for all configured OAuth2 profiles."
  categories  = ["auth"]

  annotations {
    mode = "read"
  }

  operation {
    protocol = "bash"

    bash {
      script = "earl auth status"
      sandbox {
        network = false
      }
    }
  }
}

command "shell_completion" {
  title       = "Shell completion"
  summary     = "Generate shell completion script"
  description = "Generates a shell completion script for the specified shell. Output can be sourced or saved to the appropriate completions directory."
  categories  = ["setup"]

  annotations {
    mode = "read"
  }

  param "shell" {
    type        = "string"
    required    = false
    default     = "zsh"
    description = "Shell to generate completions for: bash, zsh, fish, powershell, or elvish"
  }

  operation {
    protocol = "bash"

    bash {
      script = "earl completion {{ args.shell }}"
      sandbox {
        network = false
      }
    }
  }
}

# ---------------------------------------------------------------------------
# Write operations
# ---------------------------------------------------------------------------

command "set_secret" {
  title       = "Set secret"
  summary     = "Store a secret value in the keychain"
  description = "Stores a secret value in Earl's secure keychain, creating or overwriting the entry for the given key."
  categories  = ["secrets"]

  annotations {
    mode = "write"
  }

  param "key" {
    type        = "string"
    required    = true
    description = "Secret key name (e.g. 'github.token')"
  }

  param "value" {
    type        = "string"
    required    = true
    description = "Secret value to store"
  }

  operation {
    protocol = "bash"

    bash {
      script = "echo '{{ args.value }}' | earl secrets set '{{ args.key }}' --stdin"
      sandbox {
        network = false
      }
    }
  }
}

command "delete_secret" {
  title       = "Delete secret"
  summary     = "Delete a secret from the keychain"
  description = "Permanently removes a secret from Earl's secure keychain."
  categories  = ["secrets"]

  annotations {
    mode = "write"
  }

  param "key" {
    type        = "string"
    required    = true
    description = "Secret key name to delete"
  }

  operation {
    protocol = "bash"

    bash {
      script = "earl secrets delete '{{ args.key }}'"
      sandbox {
        network = false
      }
    }
  }
}

command "import_template" {
  title       = "Import template"
  summary     = "Import a template from a path or URL"
  description = "Imports a template file into Earl from a local file path or a direct HTTP(S) URL to an .hcl file."
  categories  = ["templates"]

  annotations {
    mode = "write"
  }

  param "source" {
    type        = "string"
    required    = true
    description = "Local file path or HTTP(S) URL to an .hcl template file"
  }

  param "scope" {
    type        = "string"
    required    = false
    default     = "local"
    description = "Import destination: 'local' or 'global'"
  }

  operation {
    protocol = "bash"

    bash {
      script = "earl templates import --scope {{ args.scope }} '{{ args.source }}'"
      sandbox {
        network = false
      }
    }
  }
}

command "auth_logout" {
  title       = "Auth logout"
  summary     = "Delete OAuth token for a profile"
  description = "Removes the stored OAuth2 token for the specified authentication profile."
  categories  = ["auth"]

  annotations {
    mode = "write"
  }

  param "profile" {
    type        = "string"
    required    = true
    description = "OAuth2 profile name"
  }

  operation {
    protocol = "bash"

    bash {
      script = "earl auth logout '{{ args.profile }}'"
      sandbox {
        network = false
      }
    }
  }
}

command "auth_refresh" {
  title       = "Auth refresh"
  summary     = "Force-refresh an OAuth token"
  description = "Forces a token refresh for the specified OAuth2 authentication profile."
  categories  = ["auth"]

  annotations {
    mode = "write"
  }

  param "profile" {
    type        = "string"
    required    = true
    description = "OAuth2 profile name"
  }

  operation {
    protocol = "bash"

    bash {
      script = "earl auth refresh '{{ args.profile }}'"
      sandbox {
        network = false
      }
    }
  }
}
