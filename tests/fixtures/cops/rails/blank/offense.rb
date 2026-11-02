items.nil? || items.empty?
^ Rails/Blank: Use `items.blank?` instead of `items.nil? || items.empty?`.
!items.present?
^ Rails/Blank: Use `items.blank?` instead of `!items.present?`.
items == nil || items.empty?
^ Rails/Blank: Use `items.blank?` instead of `items == nil || items.empty?`.
