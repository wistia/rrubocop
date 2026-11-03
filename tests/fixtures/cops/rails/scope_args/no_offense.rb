scope :active, -> { where(active: true) }
self.scope :active, where(active: true)
