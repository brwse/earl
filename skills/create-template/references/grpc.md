# gRPC Templates

Use gRPC when calling gRPC services (protobuf-based RPC).

## Template Skeleton (with reflection)

```hcl
version = 1
provider = "myservice"

command "health_check" {
  title       = "Health Check"
  summary     = "Check gRPC service health"
  description = <<-EOT
    Calls the standard gRPC health check endpoint to verify service availability.

    ## Guidance for AI agents

    Use this command to check whether the gRPC service is running and healthy. Leave service empty to check server-wide health.

    Example: `earl call myservice.health_check --service ""`
  EOT

  annotations {
    mode = "read"
  }

  param "service" {
    type        = "string"
    required    = false
    description = "Service name to check (empty for server health)"
    default     = ""
  }

  operation {
    protocol   = "grpc"
    url        = "http://grpc.example.com:50051"
    timeout_ms = 5000

    grpc {
      service = "grpc.health.v1.Health"
      method  = "Check"

      body = {
        service = "{{ args.service }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Status: {{ result.status }}"
  }
}
```

## Key Fields

| Field                      | Required | Description                                 |
| -------------------------- | -------- | ------------------------------------------- |
| `url`                      | Yes      | gRPC server address (include port)          |
| `timeout_ms`               | No       | Request timeout in milliseconds             |
| `grpc.service`             | Yes      | Fully qualified service name                |
| `grpc.method`              | Yes      | RPC method name                             |
| `grpc.body`                | No       | Request message as key-value map            |
| `grpc.descriptor_set_file` | No       | Path to offline descriptor set (`.fds.bin`) |

**Note:** gRPC uses a nested `grpc` block inside `operation`.

## Reflection vs Descriptor Set

**Reflection (default):** Earl discovers the service schema at runtime via gRPC reflection. The server must support reflection (v1). No extra files needed.

**Descriptor set:** For servers without reflection, provide a pre-compiled descriptor set:

```hcl
grpc {
  service             = "mypackage.MyService"
  method              = "MyMethod"
  descriptor_set_file = "myservice.fds.bin"

  body = {
    field = "{{ args.value }}"
  }
}
```

Generate the descriptor set from proto files:

```bash
protoc --descriptor_set_out=myservice.fds.bin --include_imports myservice.proto
```

## Known Issues

**TLS with SSRF protection:** Earl's TOCTOU protection pins the gRPC endpoint to its resolved IP address, which can break TLS certificate validation (the cert expects the hostname, not the IP). For TLS endpoints, consider using `descriptor_set_file` to avoid reflection-related issues, or use HTTP-based testing.

**Reflection version:** Earl uses gRPC reflection v1. Some servers only support v1alpha. If reflection fails, try using a descriptor set instead.
