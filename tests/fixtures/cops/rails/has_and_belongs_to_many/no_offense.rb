model.has_and_belongs_to_many :roles
has_many :roles, through: :memberships
