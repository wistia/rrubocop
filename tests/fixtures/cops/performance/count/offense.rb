items.select { |item| item.active? }.count
      ^ Performance/Count: Use `count` instead of `select...count`.
items.reject { |item| item.active? }.size
      ^ Performance/Count: Use `count` instead of `reject...size`.
items.select(&:active?).length
      ^ Performance/Count: Use `count` instead of `select...length`.
