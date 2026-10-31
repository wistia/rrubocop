def query
  sql = enabled? ? sql() : ""
end
