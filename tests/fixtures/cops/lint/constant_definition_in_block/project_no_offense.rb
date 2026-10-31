configure do
  if server?
    LIMIT = 5
  end
end
configure { Namespace::LIMIT = 5 }
