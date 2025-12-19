//! Integration tests for the C FFI surface.

use clique_fusion::CHI2_2D_CONFIDENCE_95;
use clique_fusion_ffi::{
    CliqueC, CliqueIndex_cliques, CliqueIndex_free, CliqueIndex_from_observations, CliqueSetC_free,
    ObservationC,
};
use std::slice;
use uuid::Uuid;

type UuidC = [u8; 16];

const fn uuid_to_uuidc(uuid: Uuid) -> UuidC {
    *uuid.as_bytes()
}

const fn make_observation(id: Uuid, x: f64, y: f64) -> ObservationC {
    ObservationC {
        id: uuid_to_uuidc(id),
        x,
        y,
        cov_xx: 1.0,
        cov_xy: 0.0,
        cov_yy: 1.0,
        context: [0u8; 16],
    }
}

#[test]
fn test_create_insert_cliques_free() {
    let chi2 = CHI2_2D_CONFIDENCE_95;
    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();
    let id3 = Uuid::new_v4();

    let obs1 = make_observation(id1, 1.0, 1.0);
    let obs2 = make_observation(id2, 1.05, 1.05);
    let obs3 = make_observation(id3, 50.0, 50.0); // Clearly separate

    let observations = [obs1, obs2, obs3];

    let index_ptr =
        unsafe { CliqueIndex_from_observations(chi2, observations.as_ptr(), observations.len()) };

    assert!(
        !index_ptr.is_null(),
        "CliqueIndex_from_observations returned null"
    );

    let clique_set_ptr = unsafe { CliqueIndex_cliques(index_ptr) };
    assert!(
        !clique_set_ptr.is_null(),
        "CliqueIndex_cliques returned null"
    );

    let clique_set = unsafe { &*clique_set_ptr };
    let cliques: &[CliqueC] = unsafe { slice::from_raw_parts(clique_set.cliques, clique_set.len) };

    println!("Number of cliques returned: {}", cliques.len());

    for (i, clique) in cliques.iter().enumerate() {
        println!("Clique {i} length: {}", clique.len);
        let ids: &[[u8; 16]] = unsafe { slice::from_raw_parts(clique.uuids, clique.len) };
        for uuid_bytes in ids {
            let uuid = Uuid::from_bytes(*uuid_bytes);
            println!(" - {uuid:?}");
        }
    }

    // Should be 1 clique: [id1, id2] (singular cliques are not returned)
    assert_eq!(cliques.len(), 1);

    let mut all_uuids: Vec<Uuid> = vec![];
    for clique in cliques {
        let ids: &[[u8; 16]] = unsafe { slice::from_raw_parts(clique.uuids, clique.len) };
        for id_bytes in ids {
            let uuid = Uuid::from_bytes(*id_bytes);
            all_uuids.push(uuid);
        }
    }

    assert!(all_uuids.contains(&id1));
    assert!(all_uuids.contains(&id2));
    assert!(!all_uuids.contains(&id3)); // not present- singleton clique

    unsafe {
        CliqueSetC_free(clique_set_ptr);
        CliqueIndex_free(index_ptr);
    }
}
