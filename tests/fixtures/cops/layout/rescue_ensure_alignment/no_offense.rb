def truncate
  ClickHouse::Record.connection
                    .tables
                    .each do |table|
    ClickHouse::Record.connection.execute("TRUNCATE #{table}")
  rescue ActiveRecord::ActiveRecordError
    next
  end
end

begin
  work
rescue
  handle
end

# Method chain: rescue aligned with leading dot on the `do` line.
Authentication
  .where(provider: :barong)
  .each do |auth|
    auth.touch
  rescue => e
    report_exception(e)
  end

Fiber
  .new do
    work
  rescue => e
    handle(e)
  end

# Dot on same line as do: rescue aligned with that dot.
OT.with_diagnostics do
    Sentry.close
  rescue ThreadError
    warn "interrupted"
end

# Method name on do line: rescue aligned with the selector.
OT.with_diagnostics do
    Sentry.close
   rescue ThreadError
     warn "interrupted"
end

# Leading-dot chain: ensure aligned with the dot.
Fiber
  .new do
    work
  ensure
    cleanup
  end

value = begin; parse(output); rescue ParseError; nil end
