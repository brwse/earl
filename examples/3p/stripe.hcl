version = 1
provider = "stripe"
categories = ["payments", "billing", "commerce"]

command "list_customers" {
  title       = "List customers"
  summary     = "List Stripe customers with optional filters"
  description = "Retrieve a paginated list of customers. Filter by email or creation date."
  categories  = ["customers"]

  annotations {
    mode    = "read"
    secrets = ["stripe.api_key"]
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 10
    description = "Number of results to return (1-100)"
  }

  param "email" {
    type        = "string"
    required    = false
    description = "Filter by exact email address (case-sensitive)"
  }

  param "starting_after" {
    type        = "string"
    required    = false
    description = "Cursor for pagination: customer ID to start after"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.stripe.com/v1/customers"

    auth {
      kind   = "bearer"
      secret = "stripe.api_key"
    }

    query = {
      limit          = "{{ args.limit }}"
      email          = "{{ args.email }}"
      starting_after = "{{ args.starting_after }}"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result.data | length }} customers (has_more: {{ result.has_more }}).\n{% for c in result.data %}- {{ c.id }}: {{ c.name | default('(no name)') }} <{{ c.email | default('(no email)') }}>\n{% endfor %}"
  }
}

command "get_customer" {
  title       = "Get customer"
  summary     = "Retrieve a single customer by ID"
  description = "Fetch full details for a Stripe customer by their ID."
  categories  = ["customers"]

  annotations {
    mode    = "read"
    secrets = ["stripe.api_key"]
  }

  param "customer_id" {
    type        = "string"
    required    = true
    description = "Stripe customer ID (e.g. cus_xxx)"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.stripe.com/v1/customers/{{ args.customer_id }}"

    auth {
      kind   = "bearer"
      secret = "stripe.api_key"
    }
  }

  result {
    decode = "json"
    output = "Customer {{ result.id }}:\n  Name: {{ result.name | default('(not set)') }}\n  Email: {{ result.email | default('(not set)') }}\n  Phone: {{ result.phone | default('(not set)') }}\n  Balance: {{ result.balance }} {{ result.currency | default('usd') }}\n  Created: {{ result.created }}\n  Default source: {{ result.default_source | default('none') }}"
  }
}

command "create_customer" {
  title       = "Create customer"
  summary     = "Create a new Stripe customer"
  description = "Create a customer record in Stripe with optional name, email, phone, and description."
  categories  = ["customers"]

  annotations {
    mode    = "write"
    secrets = ["stripe.api_key"]
  }

  param "name" {
    type        = "string"
    required    = false
    description = "Customer's full name"
  }

  param "email" {
    type        = "string"
    required    = false
    description = "Customer's email address"
  }

  param "phone" {
    type        = "string"
    required    = false
    description = "Customer's phone number"
  }

  param "description" {
    type        = "string"
    required    = false
    description = "Arbitrary description for the customer"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.stripe.com/v1/customers"

    auth {
      kind   = "bearer"
      secret = "stripe.api_key"
    }

    body {
      kind = "form_urlencoded"
      fields = {
        name        = "{{ args.name }}"
        email       = "{{ args.email }}"
        phone       = "{{ args.phone }}"
        description = "{{ args.description }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Created customer {{ result.id }}:\n  Name: {{ result.name | default('(not set)') }}\n  Email: {{ result.email | default('(not set)') }}"
  }
}

command "update_customer" {
  title       = "Update customer"
  summary     = "Update an existing customer's details"
  description = "Update fields on an existing Stripe customer by ID."
  categories  = ["customers"]

  annotations {
    mode    = "write"
    secrets = ["stripe.api_key"]
  }

  param "customer_id" {
    type        = "string"
    required    = true
    description = "Stripe customer ID to update"
  }

  param "name" {
    type        = "string"
    required    = false
    description = "Updated full name"
  }

  param "email" {
    type        = "string"
    required    = false
    description = "Updated email address"
  }

  param "phone" {
    type        = "string"
    required    = false
    description = "Updated phone number"
  }

  param "description" {
    type        = "string"
    required    = false
    description = "Updated description"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.stripe.com/v1/customers/{{ args.customer_id }}"

    auth {
      kind   = "bearer"
      secret = "stripe.api_key"
    }

    body {
      kind = "form_urlencoded"
      fields = {
        name        = "{{ args.name }}"
        email       = "{{ args.email }}"
        phone       = "{{ args.phone }}"
        description = "{{ args.description }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Updated customer {{ result.id }}:\n  Name: {{ result.name | default('(not set)') }}\n  Email: {{ result.email | default('(not set)') }}"
  }
}

command "create_payment_intent" {
  title       = "Create payment intent"
  summary     = "Create a new PaymentIntent for collecting a payment"
  description = "Create a PaymentIntent to initiate a payment flow. Amount is in smallest currency unit (e.g. cents for USD)."
  categories  = ["payments"]

  annotations {
    mode    = "write"
    secrets = ["stripe.api_key"]
  }

  param "amount" {
    type        = "integer"
    required    = true
    description = "Amount in smallest currency unit (e.g. 1000 = $10.00 USD)"
  }

  param "currency" {
    type        = "string"
    required    = true
    description = "Three-letter ISO currency code, lowercase (e.g. usd, eur)"
  }

  param "customer" {
    type        = "string"
    required    = false
    description = "Customer ID to associate the payment with"
  }

  param "description" {
    type        = "string"
    required    = false
    description = "Arbitrary description for the payment"
  }

  param "confirm" {
    type        = "boolean"
    required    = false
    default     = false
    description = "Whether to confirm the PaymentIntent immediately"
  }

  param "payment_method" {
    type        = "string"
    required    = false
    description = "PaymentMethod ID to use for this payment"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.stripe.com/v1/payment_intents"

    auth {
      kind   = "bearer"
      secret = "stripe.api_key"
    }

    body {
      kind = "form_urlencoded"
      fields = {
        amount         = "{{ args.amount }}"
        currency       = "{{ args.currency }}"
        customer       = "{{ args.customer }}"
        description    = "{{ args.description }}"
        confirm        = "{{ args.confirm }}"
        payment_method = "{{ args.payment_method }}"
      }
    }
  }

  result {
    decode = "json"
    output = "PaymentIntent {{ result.id }}:\n  Status: {{ result.status }}\n  Amount: {{ result.amount }} {{ result.currency }}\n  Customer: {{ result.customer | default('none') }}\n  Client secret: {{ result.client_secret }}"
  }
}

command "list_payment_intents" {
  title       = "List payment intents"
  summary     = "List PaymentIntents with optional filters"
  description = "Retrieve a paginated list of PaymentIntents. Filter by customer or creation date."
  categories  = ["payments"]

  annotations {
    mode    = "read"
    secrets = ["stripe.api_key"]
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 10
    description = "Number of results to return (1-100)"
  }

  param "customer" {
    type        = "string"
    required    = false
    description = "Filter by customer ID"
  }

  param "starting_after" {
    type        = "string"
    required    = false
    description = "Cursor for pagination: PaymentIntent ID to start after"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.stripe.com/v1/payment_intents"

    auth {
      kind   = "bearer"
      secret = "stripe.api_key"
    }

    query = {
      limit          = "{{ args.limit }}"
      customer       = "{{ args.customer }}"
      starting_after = "{{ args.starting_after }}"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result.data | length }} payment intents (has_more: {{ result.has_more }}).\n{% for pi in result.data %}- {{ pi.id }}: {{ pi.status }} — {{ pi.amount }} {{ pi.currency }}\n{% endfor %}"
  }
}

command "create_refund" {
  title       = "Create refund"
  summary     = "Refund a payment in full or partially"
  description = "Create a refund for a PaymentIntent or Charge. Omit amount for a full refund."
  categories  = ["payments"]

  annotations {
    mode    = "write"
    secrets = ["stripe.api_key"]
  }

  param "payment_intent" {
    type        = "string"
    required    = true
    description = "PaymentIntent ID to refund (e.g. pi_xxx)"
  }

  param "amount" {
    type        = "integer"
    required    = false
    description = "Partial refund amount in smallest currency unit; omit for full refund"
  }

  param "reason" {
    type        = "string"
    required    = false
    description = "Reason: duplicate, fraudulent, or requested_by_customer"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.stripe.com/v1/refunds"

    auth {
      kind   = "bearer"
      secret = "stripe.api_key"
    }

    body {
      kind = "form_urlencoded"
      fields = {
        payment_intent = "{{ args.payment_intent }}"
        amount         = "{{ args.amount }}"
        reason         = "{{ args.reason }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Refund {{ result.id }}:\n  Status: {{ result.status }}\n  Amount: {{ result.amount }} {{ result.currency }}\n  Reason: {{ result.reason | default('none') }}\n  Payment intent: {{ result.payment_intent | default('n/a') }}"
  }
}

command "list_products" {
  title       = "List products"
  summary     = "List products in your Stripe catalog"
  description = "Retrieve a paginated list of products. Optionally filter by active status."
  categories  = ["products"]

  annotations {
    mode    = "read"
    secrets = ["stripe.api_key"]
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 10
    description = "Number of results to return (1-100)"
  }

  param "active" {
    type        = "boolean"
    required    = false
    description = "Filter by active status (true or false)"
  }

  param "starting_after" {
    type        = "string"
    required    = false
    description = "Cursor for pagination: product ID to start after"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.stripe.com/v1/products"

    auth {
      kind   = "bearer"
      secret = "stripe.api_key"
    }

    query = {
      limit          = "{{ args.limit }}"
      active         = "{{ args.active }}"
      starting_after = "{{ args.starting_after }}"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result.data | length }} products (has_more: {{ result.has_more }}).\n{% for p in result.data %}- {{ p.id }}: {{ p.name }} (active: {{ p.active }}){% if p.description %} — {{ p.description }}{% endif %}\n{% endfor %}"
  }
}

command "create_product" {
  title       = "Create product"
  summary     = "Create a new product in Stripe"
  description = "Create a product in your catalog. Products can be used with Prices for checkout or subscriptions."
  categories  = ["products"]

  annotations {
    mode    = "write"
    secrets = ["stripe.api_key"]
  }

  param "name" {
    type        = "string"
    required    = true
    description = "Product name, displayed to the customer"
  }

  param "description" {
    type        = "string"
    required    = false
    description = "Product description"
  }

  param "active" {
    type        = "boolean"
    required    = false
    default     = true
    description = "Whether the product is available for purchase"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.stripe.com/v1/products"

    auth {
      kind   = "bearer"
      secret = "stripe.api_key"
    }

    body {
      kind = "form_urlencoded"
      fields = {
        name        = "{{ args.name }}"
        description = "{{ args.description }}"
        active      = "{{ args.active }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Created product {{ result.id }}:\n  Name: {{ result.name }}\n  Active: {{ result.active }}\n  Description: {{ result.description | default('(none)') }}"
  }
}

command "list_invoices" {
  title       = "List invoices"
  summary     = "List invoices with optional filters"
  description = "Retrieve a paginated list of invoices. Filter by customer, status, or subscription."
  categories  = ["billing"]

  annotations {
    mode    = "read"
    secrets = ["stripe.api_key"]
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 10
    description = "Number of results to return (1-100)"
  }

  param "customer" {
    type        = "string"
    required    = false
    description = "Filter by customer ID"
  }

  param "status" {
    type        = "string"
    required    = false
    description = "Filter by status: draft, open, paid, uncollectible, or void"
  }

  param "subscription" {
    type        = "string"
    required    = false
    description = "Filter by subscription ID"
  }

  param "starting_after" {
    type        = "string"
    required    = false
    description = "Cursor for pagination: invoice ID to start after"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.stripe.com/v1/invoices"

    auth {
      kind   = "bearer"
      secret = "stripe.api_key"
    }

    query = {
      limit          = "{{ args.limit }}"
      customer       = "{{ args.customer }}"
      status         = "{{ args.status }}"
      subscription   = "{{ args.subscription }}"
      starting_after = "{{ args.starting_after }}"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result.data | length }} invoices (has_more: {{ result.has_more }}).\n{% for inv in result.data %}- {{ inv.id }}: {{ inv.status }} — {{ inv.amount_due }} {{ inv.currency }} (customer: {{ inv.customer }})\n{% endfor %}"
  }
}

command "create_subscription" {
  title       = "Create subscription"
  summary     = "Subscribe a customer to a recurring price"
  description = "Create a subscription for a customer with a given price. Optionally set a trial period or quantity."
  categories  = ["billing"]

  annotations {
    mode    = "write"
    secrets = ["stripe.api_key"]
  }

  param "customer" {
    type        = "string"
    required    = true
    description = "Customer ID to subscribe"
  }

  param "price" {
    type        = "string"
    required    = true
    description = "Price ID for the subscription item (e.g. price_xxx)"
  }

  param "quantity" {
    type        = "integer"
    required    = false
    default     = 1
    description = "Quantity for the subscription item"
  }

  param "trial_period_days" {
    type        = "integer"
    required    = false
    description = "Number of free trial days before first charge"
  }

  operation {
    protocol = "http"
    method   = "POST"
    url      = "https://api.stripe.com/v1/subscriptions"

    auth {
      kind   = "bearer"
      secret = "stripe.api_key"
    }

    body {
      kind = "form_urlencoded"
      fields = {
        customer            = "{{ args.customer }}"
        "items[0][price]"   = "{{ args.price }}"
        "items[0][quantity]" = "{{ args.quantity }}"
        trial_period_days   = "{{ args.trial_period_days }}"
      }
    }
  }

  result {
    decode = "json"
    output = "Created subscription {{ result.id }}:\n  Status: {{ result.status }}\n  Customer: {{ result.customer }}\n  Current period: {{ result.current_period_start }} — {{ result.current_period_end }}\n  Collection: {{ result.collection_method }}"
  }
}

command "list_subscriptions" {
  title       = "List subscriptions"
  summary     = "List subscriptions with optional filters"
  description = "Retrieve a paginated list of subscriptions. Filter by customer or status."
  categories  = ["billing"]

  annotations {
    mode    = "read"
    secrets = ["stripe.api_key"]
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 10
    description = "Number of results to return (1-100)"
  }

  param "customer" {
    type        = "string"
    required    = false
    description = "Filter by customer ID"
  }

  param "status" {
    type        = "string"
    required    = false
    description = "Filter: active, past_due, canceled, unpaid, trialing, or all"
  }

  param "starting_after" {
    type        = "string"
    required    = false
    description = "Cursor for pagination: subscription ID to start after"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.stripe.com/v1/subscriptions"

    auth {
      kind   = "bearer"
      secret = "stripe.api_key"
    }

    query = {
      limit          = "{{ args.limit }}"
      customer       = "{{ args.customer }}"
      status         = "{{ args.status }}"
      starting_after = "{{ args.starting_after }}"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result.data | length }} subscriptions (has_more: {{ result.has_more }}).\n{% for sub in result.data %}- {{ sub.id }}: {{ sub.status }} (customer: {{ sub.customer }})\n{% endfor %}"
  }
}

command "get_balance" {
  title       = "Get balance"
  summary     = "Retrieve your Stripe account balance"
  description = "Fetch the current balance for your Stripe account, showing available and pending amounts by currency."
  categories  = ["reporting"]

  annotations {
    mode    = "read"
    secrets = ["stripe.api_key"]
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.stripe.com/v1/balance"

    auth {
      kind   = "bearer"
      secret = "stripe.api_key"
    }
  }

  result {
    decode = "json"
    output = "Stripe Balance:\n{% for b in result.available %}  Available: {{ b.amount }} {{ b.currency }}\n{% endfor %}{% for b in result.pending %}  Pending: {{ b.amount }} {{ b.currency }}\n{% endfor %}"
  }
}

command "list_balance_transactions" {
  title       = "List balance transactions"
  summary     = "List balance transactions with optional filters"
  description = "Retrieve a paginated list of balance transactions (charges, refunds, payouts, etc.)."
  categories  = ["reporting"]

  annotations {
    mode    = "read"
    secrets = ["stripe.api_key"]
  }

  param "limit" {
    type        = "integer"
    required    = false
    default     = 10
    description = "Number of results to return (1-100)"
  }

  param "type" {
    type        = "string"
    required    = false
    description = "Filter by type: charge, refund, transfer, payout, etc."
  }

  param "currency" {
    type        = "string"
    required    = false
    description = "Filter by three-letter ISO currency code, lowercase"
  }

  param "starting_after" {
    type        = "string"
    required    = false
    description = "Cursor for pagination: transaction ID to start after"
  }

  operation {
    protocol = "http"
    method   = "GET"
    url      = "https://api.stripe.com/v1/balance_transactions"

    auth {
      kind   = "bearer"
      secret = "stripe.api_key"
    }

    query = {
      limit          = "{{ args.limit }}"
      type           = "{{ args.type }}"
      currency       = "{{ args.currency }}"
      starting_after = "{{ args.starting_after }}"
    }
  }

  result {
    decode = "json"
    output = "Found {{ result.data | length }} transactions (has_more: {{ result.has_more }}).\n{% for txn in result.data %}- {{ txn.id }}: {{ txn.type }} — {{ txn.amount }} {{ txn.currency }} (net: {{ txn.net }}, fee: {{ txn.fee }})\n{% endfor %}"
  }
}
