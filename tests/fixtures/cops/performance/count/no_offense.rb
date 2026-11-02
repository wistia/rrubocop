items.count { |item| item.active? }
Model.select(:name).count
items.select { |item| item.active? }.count { |item| item.ready? }
items.select.count
