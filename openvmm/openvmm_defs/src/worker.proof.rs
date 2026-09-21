// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

use super::SavedState;
use vstd::prelude::*;

verus! {

// Base digits are Unicode scalar values plus one; zero terminates the name.
pub closed spec fn component_name_id(name: Seq<char>) -> nat
    decreases name.len(),
{
    if name.len() == 0 {
        0
    } else {
        1 + name[0] as nat + 0x110001 * component_name_id(name.drop_first())
    }
}

pub proof fn component_name_identity(left: Seq<char>, right: Seq<char>)
    ensures
        component_name_id(left) == component_name_id(right) <==> left == right,
    decreases left.len() + right.len(),
{
    reveal(component_name_id);
    if component_name_id(left) == component_name_id(right) {
        if left.len() != 0 && right.len() != 0 {
            assert(0 <= left[0] as int <= 0x10ffff);
            assert(0 <= right[0] as int <= 0x10ffff);
            assert(left[0] == right[0]);
            assert(component_name_id(left.drop_first()) == component_name_id(right.drop_first()));
            component_name_identity(left.drop_first(), right.drop_first());
            assert forall|i: int| 0 <= i < left.len() implies #[trigger] left[i] == right[i] by {
                if i > 0 {
                    assert(left[i] == left.drop_first()[i - 1]);
                    assert(right[i] == right.drop_first()[i - 1]);
                }
            }
            assert(left =~= right);
        } else {
            assert(left =~= right);
        }
    }
}

pub closed spec fn inventory_ids(inventory: Seq<String>) -> Set<nat> {
    inventory.map_values(|name: String| component_name_id(name@)).to_set()
}

pub closed spec fn saved_inventory_ids(saved_state: &SavedState) -> Set<nat> {
    inventory_ids(saved_state.inventory@)
}

pub proof fn inventory_id_membership(inventory: Seq<String>, id: nat)
    ensures
        inventory_ids(inventory).contains(id) <==> exists|i: int|
            0 <= i < inventory.len() && #[trigger] component_name_id(inventory[i]@) == id,
{
    reveal(inventory_ids);
    let ids = inventory.map_values(|name: String| component_name_id(name@));
    ids.to_set_ensures();
    if ids.contains(id) {
        let i = choose|i: int| 0 <= i < ids.len() && #[trigger] ids[i] == id;
        assert(component_name_id(inventory[i]@) == id);
    } else if exists|i: int|
        0 <= i < inventory.len() && #[trigger] component_name_id(inventory[i]@) == id {
        let i = choose|i: int|
            0 <= i < inventory.len() && #[trigger] component_name_id(inventory[i]@) == id;
        assert(ids[i] == id);
    }
}

pub proof fn inventory_membership(inventory: Seq<String>, name: Seq<char>)
    ensures
        inventory_ids(inventory).contains(component_name_id(name)) <==> exists|i: int|
            0 <= i < inventory.len() && #[trigger] inventory[i]@ == name,
{
    inventory_id_membership(inventory, component_name_id(name));
    if inventory_ids(inventory).contains(component_name_id(name)) {
        let i = choose|i: int|
            0 <= i < inventory.len() && #[trigger] component_name_id(inventory[i]@)
                == component_name_id(name);
        component_name_identity(inventory[i]@, name);
    } else if exists|i: int| 0 <= i < inventory.len() && #[trigger] inventory[i]@ == name {
        let i = choose|i: int| 0 <= i < inventory.len() && #[trigger] inventory[i]@ == name;
        assert(component_name_id(inventory[i]@) == component_name_id(name));
    }
}

pub proof fn saved_inventory_membership(saved_state: &SavedState, name: Seq<char>)
    ensures
        saved_inventory_ids(saved_state).contains(component_name_id(name)) <==> exists|i: int|
            0 <= i < saved_state.inventory@.len() && #[trigger] saved_state.inventory@[i]@ == name,
{
    reveal(saved_inventory_ids);
    inventory_membership(saved_state.inventory@, name);
}

pub proof fn saved_inventory_id_membership(saved_state: &SavedState, id: nat)
    ensures
        saved_inventory_ids(saved_state).contains(id) <==> exists|i: int|
            0 <= i < saved_state.inventory@.len() && #[trigger] component_name_id(
                saved_state.inventory@[i]@,
            ) == id,
{
    reveal(saved_inventory_ids);
    inventory_id_membership(saved_state.inventory@, id);
}

pub proof fn inventory_empty()
    ensures
        inventory_ids(Seq::<String>::empty()) == Set::<nat>::empty(),
{
    reveal(inventory_ids);
    Seq::<nat>::empty().to_set_ensures();
    assert(inventory_ids(Seq::<String>::empty()) =~= Set::<nat>::empty());
}

pub proof fn saved_inventory_empty(saved_state: &SavedState)
    requires
        saved_state.inventory@.len() == 0,
    ensures
        saved_inventory_ids(saved_state) == Set::<nat>::empty(),
{
    reveal(saved_inventory_ids);
    assert(saved_state.inventory@ =~= Seq::<String>::empty());
    inventory_empty();
}

pub proof fn inventory_push(inventory: Seq<String>, name: String)
    ensures
        inventory_ids(inventory.push(name)) == inventory_ids(inventory).insert(
            component_name_id(name@),
        ),
{
    assert forall|id: nat| #[trigger]
        inventory_ids(inventory.push(name)).contains(id) <==> inventory_ids(inventory).insert(
            component_name_id(name@),
        ).contains(id) by {
        inventory_id_membership(inventory.push(name), id);
        inventory_id_membership(inventory, id);
        if inventory_ids(inventory.push(name)).contains(id) {
            let i = choose|i: int|
                0 <= i < inventory.push(name).len() && #[trigger] component_name_id(
                    inventory.push(name)[i]@,
                ) == id;
            if i < inventory.len() {
                assert(component_name_id(inventory[i]@) == id);
            }
        } else if inventory_ids(inventory).contains(id) {
            let i = choose|i: int|
                0 <= i < inventory.len() && #[trigger] component_name_id(inventory[i]@) == id;
            assert(component_name_id(inventory.push(name)[i]@) == id);
        } else if id == component_name_id(name@) {
            assert(component_name_id(inventory.push(name)[inventory.len() as int]@) == id);
        }
    }
    assert(inventory_ids(inventory.push(name)) =~= inventory_ids(inventory).insert(
        component_name_id(name@),
    ));
}

pub proof fn inventory_repeated_name(inventory: Seq<String>, name: String)
    requires
        inventory_ids(inventory).contains(component_name_id(name@)),
    ensures
        inventory_ids(inventory.push(name)) == inventory_ids(inventory),
{
    inventory_push(inventory, name);
    assert(inventory_ids(inventory).insert(component_name_id(name@)) =~= inventory_ids(inventory));
}

pub proof fn inventory_distinct_names(left: String, right: String)
    requires
        left@ != right@,
    ensures
        component_name_id(left@) != component_name_id(right@),
        inventory_ids(seq![left, right]) == Set::<nat>::empty().insert(
            component_name_id(left@),
        ).insert(component_name_id(right@)),
{
    component_name_identity(left@, right@);
    inventory_empty();
    inventory_push(Seq::empty(), left);
    assert(Seq::empty().push(left) =~= seq![left]);
    inventory_push(seq![left], right);
    assert(seq![left].push(right) =~= seq![left, right]);
}

} // verus!
