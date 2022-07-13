class CreateProposals < ActiveRecord::Migration[7.0]
  def change
    create_table :proposals do |t|

      t.timestamps
    end
  end
end
