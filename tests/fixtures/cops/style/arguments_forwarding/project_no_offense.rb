class_methods do
  def process(**kwargs)
    target(**kwargs)
  end
end
def process(**kwargs)
  target(**kwargs).map { |x| x }
end
