version = 1
provider = "envtest"

environments {
  default = "production"
  secrets = []
  production {
    base_url = "https://prod.example.com"
    label    = "prod"
  }
  staging {
    base_url = "https://staging.example.com"
    label    = "stg"
  }
}

command "echo_env" {
  title       = "Echo env"
  summary     = "Returns which environment is active"
  description = "Returns the environment label from vars."
  annotations {
    mode    = "read"
    secrets = []
  }
  operation {
    protocol = "bash"
    bash {
      script = "echo {{ vars.label }}"
    }
  }
  result {
    decode = "text"
    output = "{{ result }}"
  }
}

command "override_in_staging" {
  title       = "Override"
  summary     = "Uses a different script in staging"
  description = "HTTP in production, bash in staging."
  annotations {
    mode                                 = "read"
    secrets                              = []
    allow_environment_protocol_switching = true
  }
  operation {
    protocol = "bash"
    bash {
      script = "echo production"
    }
  }
  environment "staging" {
    operation {
      protocol = "bash"
      bash {
        script = "echo staging_override"
      }
    }
  }
  result {
    decode = "text"
    output = "{{ result }}"
  }
}
