items.blank?
items.nil? || other.empty?
items.empty?
items.nil? && items.empty?
def blank?
  !present?
end
