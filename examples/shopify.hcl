version = 1
provider = "shopify"
categories = ["ecommerce", "commerce"]

command "get_shop" {
  title       = "Get shop info"
  summary     = "Get store configuration and metadata"
  description = "Retrieve shop details including name, owner, plan, currency, and contact information. Requires the SHOPIFY_STORE environment variable to be set to your store subdomain."
  categories  = ["shop"]

  annotations {
    mode    = "read"
    secrets = ["shopify.access_token"]
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://{{ env.SHOPIFY_STORE }}.myshopify.com/admin/api/2025-01/shop.json"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "X-Shopify-Access-Token"
      secret   = "shopify.access_token"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = <<-EOF
    Store: {{ result.shop.name }} ({{ result.shop.myshopify_domain }})
    Owner: {{ result.shop.shop_owner }} <{{ result.shop.email }}>
    Plan: {{ result.shop.plan_display_name }}
    Currency: {{ result.shop.currency }}
    Country: {{ result.shop.country_name }}
    EOF
  }
}

command "list_products" {
  title       = "List products"
  summary     = "List products in the store"
  description = "Retrieve a list of products with optional filters for status and result limit."
  categories  = ["products"]

  annotations {
    mode    = "read"
    secrets = ["shopify.access_token"]
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 50
    description = "Maximum number of results (max 250)"
  }

  param "status" {
    type        = "string"
    required    = false
    description = "Filter by status: active, draft, or archived"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://{{ env.SHOPIFY_STORE }}.myshopify.com/admin/api/2025-01/products.json"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "X-Shopify-Access-Token"
      secret   = "shopify.access_token"
    }

    query = {
      limit  = "{{ args.limit }}"
      status = "{{ args.status }}"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = <<-EOF
    {% for p in result.products %}
    [{{ p.id }}] {{ p.title }} — {{ p.status }} — {{ p.vendor }} — {{ p.variants[0].price }} ({{ p.variants | length }} variants)
    {% endfor %}
    Found {{ result.products | length }} products.
    EOF
  }
}

command "get_product" {
  title       = "Get product"
  summary     = "Get a single product by ID"
  description = "Retrieve detailed information about a specific product including all its variants."
  categories  = ["products"]

  annotations {
    mode    = "read"
    secrets = ["shopify.access_token"]
  }

  param "product_id" {
    type        = "integer"
    required    = true
    description = "Product ID"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://{{ env.SHOPIFY_STORE }}.myshopify.com/admin/api/2025-01/products/{{ args.product_id }}.json"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "X-Shopify-Access-Token"
      secret   = "shopify.access_token"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = <<-EOF
    Product: {{ result.product.title }} [{{ result.product.id }}]
    Status: {{ result.product.status }}
    Vendor: {{ result.product.vendor }}
    Type: {{ result.product.product_type }}
    Tags: {{ result.product.tags }}
    Created: {{ result.product.created_at }}

    Variants:
    {% for v in result.product.variants %}
      - {{ v.title }}: {{ v.price }} (SKU: {{ v.sku }}, Stock: {{ v.inventory_quantity }})
    {% endfor %}
    EOF
  }
}

command "create_product" {
  title       = "Create product"
  summary     = "Create a new product"
  description = "Create a new product in the store with a title, description, vendor, and other details."
  categories  = ["products"]

  annotations {
    mode    = "write"
    secrets = ["shopify.access_token"]
  }

  param "title" {
    type        = "string"
    required    = true
    description = "Product name"
  }

  param "body_html" {
    type        = "string"
    required    = false
    default     = ""
    description = "HTML description"
  }

  param "vendor" {
    type        = "string"
    required    = false
    default     = ""
    description = "Manufacturer or vendor"
  }

  param "product_type" {
    type        = "string"
    required    = false
    default     = ""
    description = "Product category"
  }

  param "status" {
    type        = "string"
    required    = false
    default     = "active"
    description = "Product status: active, draft, or archived"
  }

  param "tags" {
    type        = "string"
    required    = false
    default     = ""
    description = "Comma-separated tags"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://{{ env.SHOPIFY_STORE }}.myshopify.com/admin/api/2025-01/products.json"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "X-Shopify-Access-Token"
      secret   = "shopify.access_token"
    }

    headers = {
      Accept = "application/json"
    }

    body {
      kind = "json"
      value = {
        product = {
          title        = "{{ args.title }}"
          body_html    = "{{ args.body_html }}"
          vendor       = "{{ args.vendor }}"
          product_type = "{{ args.product_type }}"
          status       = "{{ args.status }}"
          tags         = "{{ args.tags }}"
        }
      }
    }
  }

  result {
    decode = "json"
    output = <<-EOF
    Created product: {{ result.product.title }} [{{ result.product.id }}]
    Handle: {{ result.product.handle }}
    Status: {{ result.product.status }}
    EOF
  }
}

command "update_product" {
  title       = "Update product"
  summary     = "Update an existing product"
  description = "Update fields on an existing product. Only provide the fields you want to change."
  categories  = ["products"]

  annotations {
    mode    = "write"
    secrets = ["shopify.access_token"]
  }

  param "product_id" {
    type        = "integer"
    required    = true
    description = "Product ID to update"
  }

  param "title" {
    type        = "string"
    required    = false
    description = "New title"
  }

  param "body_html" {
    type        = "string"
    required    = false
    description = "New HTML description"
  }

  param "vendor" {
    type        = "string"
    required    = false
    description = "New vendor"
  }

  param "product_type" {
    type        = "string"
    required    = false
    description = "New product type"
  }

  param "status" {
    type        = "string"
    required    = false
    description = "New status: active, draft, or archived"
  }

  param "tags" {
    type        = "string"
    required    = false
    description = "Comma-separated tags"
  }

  operation {
    protocol = "http"
    method   = "PUT"
    url      = "https://{{ env.SHOPIFY_STORE }}.myshopify.com/admin/api/2025-01/products/{{ args.product_id }}.json"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "X-Shopify-Access-Token"
      secret   = "shopify.access_token"
    }

    headers = {
      Accept = "application/json"
    }

    body {
      kind = "json"
      value = {
        product = {
          title        = "{{ args.title | default('') }}"
          body_html    = "{{ args.body_html | default('') }}"
          vendor       = "{{ args.vendor | default('') }}"
          product_type = "{{ args.product_type | default('') }}"
          status       = "{{ args.status | default('') }}"
          tags         = "{{ args.tags | default('') }}"
        }
      }
    }
  }

  result {
    decode = "json"
    output = <<-EOF
    Updated product: {{ result.product.title }} [{{ result.product.id }}]
    Status: {{ result.product.status }}
    Updated at: {{ result.product.updated_at }}
    EOF
  }
}

command "delete_product" {
  title       = "Delete product"
  summary     = "Delete a product"
  description = "Permanently delete a product from the store."
  categories  = ["products"]

  annotations {
    mode    = "write"
    secrets = ["shopify.access_token"]
  }

  param "product_id" {
    type        = "integer"
    required    = true
    description = "Product ID to delete"
  }

  operation {
    protocol = "http"
    method   = "DELETE"
    url      = "https://{{ env.SHOPIFY_STORE }}.myshopify.com/admin/api/2025-01/products/{{ args.product_id }}.json"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "X-Shopify-Access-Token"
      secret   = "shopify.access_token"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = "Deleted product {{ args.product_id }}."
  }
}

command "list_orders" {
  title       = "List orders"
  summary     = "List orders with optional filters"
  description = "Retrieve a list of orders filtered by status, financial status, or fulfillment status."
  categories  = ["orders"]

  annotations {
    mode    = "read"
    secrets = ["shopify.access_token"]
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 50
    description = "Maximum number of results (max 250)"
  }

  param "status" {
    type        = "string"
    required    = false
    default     = "open"
    description = "Order status: open, closed, cancelled, or any"
  }

  param "financial_status" {
    type        = "string"
    required    = false
    description = "Financial status: paid, pending, refunded, authorized, or any"
  }

  param "fulfillment_status" {
    type        = "string"
    required    = false
    description = "Fulfillment status: shipped, unshipped, partial, or any"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://{{ env.SHOPIFY_STORE }}.myshopify.com/admin/api/2025-01/orders.json"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "X-Shopify-Access-Token"
      secret   = "shopify.access_token"
    }

    query = {
      limit              = "{{ args.limit }}"
      status             = "{{ args.status }}"
      financial_status   = "{{ args.financial_status }}"
      fulfillment_status = "{{ args.fulfillment_status }}"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = <<-EOF
    {% for o in result.orders %}
    {{ o.name }} [{{ o.id }}] — {{ o.financial_status }}/{{ o.fulfillment_status | default("unfulfilled") }} — {{ o.total_price }} {{ o.currency }} — {{ o.created_at }}
      Customer: {{ o.customer.first_name }} {{ o.customer.last_name }} <{{ o.email }}>
      Items: {{ o.line_items | length }}
    {% endfor %}
    Found {{ result.orders | length }} orders.
    EOF
  }
}

command "get_order" {
  title       = "Get order"
  summary     = "Get a single order by ID"
  description = "Retrieve detailed information about a specific order including line items and shipping address."
  categories  = ["orders"]

  annotations {
    mode    = "read"
    secrets = ["shopify.access_token"]
  }

  param "order_id" {
    type        = "integer"
    required    = true
    description = "Order ID"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://{{ env.SHOPIFY_STORE }}.myshopify.com/admin/api/2025-01/orders/{{ args.order_id }}.json"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "X-Shopify-Access-Token"
      secret   = "shopify.access_token"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = <<-EOF
    Order: {{ result.order.name }} [{{ result.order.id }}]
    Status: {{ result.order.financial_status }} / {{ result.order.fulfillment_status | default("unfulfilled") }}
    Total: {{ result.order.total_price }} {{ result.order.currency }}
    Customer: {{ result.order.customer.first_name }} {{ result.order.customer.last_name }} <{{ result.order.email }}>
    Created: {{ result.order.created_at }}

    Line Items:
    {% for li in result.order.line_items %}
      - {{ li.title }} x{{ li.quantity }} — {{ li.price }} (SKU: {{ li.sku }})
    {% endfor %}

    Shipping: {{ result.order.shipping_address.address1 }}, {{ result.order.shipping_address.city }}, {{ result.order.shipping_address.province }} {{ result.order.shipping_address.zip }}
    EOF
  }
}

command "close_order" {
  title       = "Close order"
  summary     = "Close an open order"
  description = "Mark an open order as closed."
  categories  = ["orders"]

  annotations {
    mode    = "write"
    secrets = ["shopify.access_token"]
  }

  param "order_id" {
    type        = "integer"
    required    = true
    description = "Order ID to close"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://{{ env.SHOPIFY_STORE }}.myshopify.com/admin/api/2025-01/orders/{{ args.order_id }}/close.json"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "X-Shopify-Access-Token"
      secret   = "shopify.access_token"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = "Closed order {{ result.order.name }} [{{ result.order.id }}] at {{ result.order.closed_at }}."
  }
}

command "cancel_order" {
  title       = "Cancel order"
  summary     = "Cancel an order"
  description = "Cancel an order with an optional reason. Valid reasons: customer, fraud, inventory, declined, other."
  categories  = ["orders"]

  annotations {
    mode    = "write"
    secrets = ["shopify.access_token"]
  }

  param "order_id" {
    type        = "integer"
    required    = true
    description = "Order ID to cancel"
  }

  param "reason" {
    type        = "string"
    required    = false
    default     = "other"
    description = "Cancellation reason: customer, fraud, inventory, declined, or other"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://{{ env.SHOPIFY_STORE }}.myshopify.com/admin/api/2025-01/orders/{{ args.order_id }}/cancel.json"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "X-Shopify-Access-Token"
      secret   = "shopify.access_token"
    }

    headers = {
      Accept = "application/json"
    }

    body {
      kind = "json"
      value = {
        reason = "{{ args.reason }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Cancelled order {{ result.order.name }} [{{ result.order.id }}]. Reason: {{ args.reason }}."
  }
}

command "list_customers" {
  title       = "List customers"
  summary     = "List customers in the store"
  description = "Retrieve a list of customers with order count and total spend."
  categories  = ["customers"]

  annotations {
    mode    = "read"
    secrets = ["shopify.access_token"]
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 50
    description = "Maximum number of results (max 250)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://{{ env.SHOPIFY_STORE }}.myshopify.com/admin/api/2025-01/customers.json"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "X-Shopify-Access-Token"
      secret   = "shopify.access_token"
    }

    query = {
      limit = "{{ args.limit }}"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = <<-EOF
    {% for c in result.customers %}
    [{{ c.id }}] {{ c.first_name }} {{ c.last_name }} <{{ c.email }}> — {{ c.orders_count }} orders, {{ c.total_spent }} spent — {{ c.state }}
    {% endfor %}
    Found {{ result.customers | length }} customers.
    EOF
  }
}

command "search_customers" {
  title       = "Search customers"
  summary     = "Search customers by query"
  description = "Search customers using Shopify query syntax (e.g. email:bob@example.com, country:Canada, orders_count:>5)."
  categories  = ["customers"]

  annotations {
    mode    = "read"
    secrets = ["shopify.access_token"]
  }

  param "query" {
    type        = "string"
    required    = true
    description = "Search query (e.g. email:bob@example.com, first_name:Bob, orders_count:>5)"
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 50
    description = "Maximum number of results (max 250)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://{{ env.SHOPIFY_STORE }}.myshopify.com/admin/api/2025-01/customers/search.json"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "X-Shopify-Access-Token"
      secret   = "shopify.access_token"
    }

    query = {
      query = "{{ args.query }}"
      limit = "{{ args.limit }}"
    }

    headers = {
      Accept = "application/json"
    }
  }

  result {
    decode = "json"
    output = <<-EOF
    {% for c in result.customers %}
    [{{ c.id }}] {{ c.first_name }} {{ c.last_name }} <{{ c.email }}> — {{ c.orders_count }} orders, {{ c.total_spent }} spent
    {% endfor %}
    Found {{ result.customers | length }} customers matching "{{ args.query }}".
    EOF
  }
}

command "create_customer" {
  title       = "Create customer"
  summary     = "Create a new customer"
  description = "Create a new customer record with email, name, phone, and tags."
  categories  = ["customers"]

  annotations {
    mode    = "write"
    secrets = ["shopify.access_token"]
  }

  param "email" {
    type        = "string"
    required    = true
    description = "Customer email (must be unique)"
  }

  param "first_name" {
    type        = "string"
    required    = false
    default     = ""
    description = "First name"
  }

  param "last_name" {
    type        = "string"
    required    = false
    default     = ""
    description = "Last name"
  }

  param "phone" {
    type        = "string"
    required    = false
    default     = ""
    description = "Phone number in E.164 format"
  }

  param "tags" {
    type        = "string"
    required    = false
    default     = ""
    description = "Comma-separated tags"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://{{ env.SHOPIFY_STORE }}.myshopify.com/admin/api/2025-01/customers.json"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "X-Shopify-Access-Token"
      secret   = "shopify.access_token"
    }

    headers = {
      Accept = "application/json"
    }

    body {
      kind = "json"
      value = {
        customer = {
          email      = "{{ args.email }}"
          first_name = "{{ args.first_name }}"
          last_name  = "{{ args.last_name }}"
          phone      = "{{ args.phone }}"
          tags       = "{{ args.tags }}"
        }
      }
    }
  }

  result {
    decode = "json"
    output = "Created customer: {{ result.customer.first_name }} {{ result.customer.last_name }} [{{ result.customer.id }}] <{{ result.customer.email }}>"
  }
}

command "adjust_inventory" {
  title       = "Adjust inventory"
  summary     = "Adjust inventory level for an item at a location"
  description = "Adjust the available inventory quantity for an inventory item at a specific location. Use positive values to add stock and negative values to subtract."
  categories  = ["inventory"]

  annotations {
    mode    = "write"
    secrets = ["shopify.access_token"]
  }

  param "inventory_item_id" {
    type        = "integer"
    required    = true
    description = "Inventory item ID (from a product variant's inventory_item_id)"
  }

  param "location_id" {
    type        = "integer"
    required    = true
    description = "Location ID"
  }

  param "available_adjustment" {
    type        = "integer"
    required    = true
    description = "Amount to adjust (positive to add, negative to subtract)"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://{{ env.SHOPIFY_STORE }}.myshopify.com/admin/api/2025-01/inventory_levels/adjust.json"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "X-Shopify-Access-Token"
      secret   = "shopify.access_token"
    }

    headers = {
      Accept = "application/json"
    }

    body {
      kind = "json"
      value = {
        inventory_item_id    = "{{ args.inventory_item_id }}"
        location_id          = "{{ args.location_id }}"
        available_adjustment = "{{ args.available_adjustment }}"
      }
    }
  }

  result {
    decode = "json"
    output = <<-EOF
    Adjusted inventory for item {{ result.inventory_level.inventory_item_id }} at location {{ result.inventory_level.location_id }}.
    New available quantity: {{ result.inventory_level.available }}
    EOF
  }
}

command "create_webhook" {
  title       = "Create webhook"
  summary     = "Create a webhook subscription"
  description = "Subscribe to a Shopify event topic with an HTTPS callback URL. Common topics: orders/create, orders/paid, products/update, customers/create, inventory_levels/update."
  categories  = ["webhooks"]

  annotations {
    mode    = "write"
    secrets = ["shopify.access_token"]
  }

  param "topic" {
    type        = "string"
    required    = true
    description = "Event topic (e.g. orders/create, products/update, customers/create)"
  }

  param "address" {
    type        = "string"
    required    = true
    description = "HTTPS callback URL"
  }

  param "format" {
    type        = "string"
    required    = false
    default     = "json"
    description = "Payload format: json or xml"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://{{ env.SHOPIFY_STORE }}.myshopify.com/admin/api/2025-01/webhooks.json"

    auth {
      kind     = "api_key"
      location = "header"
      name     = "X-Shopify-Access-Token"
      secret   = "shopify.access_token"
    }

    headers = {
      Accept = "application/json"
    }

    body {
      kind = "json"
      value = {
        webhook = {
          topic   = "{{ args.topic }}"
          address = "{{ args.address }}"
          format  = "{{ args.format }}"
        }
      }
    }
  }

  result {
    decode = "json"
    output = "Created webhook [{{ result.webhook.id }}]: {{ result.webhook.topic }} -> {{ result.webhook.address }} ({{ result.webhook.format }})"
  }
}
