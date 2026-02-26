version = 1
provider = "system"
categories = ["system", "bash"]

command "disk_usage" {
  title       = "Check disk usage"
  summary     = "Reports disk usage for a given path"
  description = "Runs du -sh in a sandboxed bash environment to report disk usage for the specified path."
  categories  = ["system"]

  annotations {
    mode = "read"
  }

  param "path" {
    type        = "string"
    required    = true
    description = "Filesystem path to check"
  }

  operation {
    protocol = "bash"

    bash {
      script = "du -sh \"$EARL_PATH\""
      env = {
        EARL_PATH = "{{ args.path }}"
      }
      sandbox {
        network = false
      }
    }
  }
}

command "list_files" {
  title       = "List files"
  summary     = "Lists files in a directory"
  description = "Runs ls -la in a sandboxed bash environment to list files in the specified directory."
  categories  = ["system"]

  annotations {
    mode = "read"
  }

  param "path" {
    type        = "string"
    required    = false
    default     = "."
    description = "Directory path to list"
  }

  operation {
    protocol = "bash"

    bash {
      script = "ls -la \"$EARL_PATH\""
      env = {
        EARL_PATH = "{{ args.path }}"
      }
      sandbox {
        network = false
      }
    }
  }
}
