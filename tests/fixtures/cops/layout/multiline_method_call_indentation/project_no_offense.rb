def query
  return records
      .where(active: true)
end
