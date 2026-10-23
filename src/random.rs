use rand::RngExt;

use crate::person::{GROUP_COUNT, Gender, Person};

/// Assign a uniformly random group in `1..=GROUP_COUNT`.
///
/// Kept for callers that don't have roster context. The app itself uses
/// [`pick_group`], which enforces the gender and balance rules.
///
/// `rand`'s thread-local generator in this version is seeded from OS
/// entropy and driven by a 12-round ChaCha stream cipher (the same cipher
/// family used in ChaCha20-Poly1305), periodically reseeding itself — a
/// genuine CSPRNG rather than a fast/insecure PRNG, so group placement
/// cannot be predicted or biased by an observer.
pub fn random_group() -> u8 {
    rand::rng().random_range(1..=GROUP_COUNT)
}

/// Choose a group for a new person of `gender` from `school`.
///
/// Rules:
/// - A group may only hold one gender: a group is compatible if it is empty
///   or every current member has `gender`. No boy and girl share a group.
/// - Among compatible groups, only the smallest ones are considered, so group
///   sizes stay similar.
/// - Among those, groups holding the fewest people from `school` are preferred
///   (spreads each school across groups).
/// - Remaining ties are broken uniformly at random from the CSPRNG.
///
/// Returns `None` when no compatible group exists (every group is already
/// held by the other gender).
pub fn pick_group(people: &[Person], gender: Gender, school: &str) -> Option<u8> {
    let mut candidates: Vec<(u8, usize, usize)> = Vec::new(); // (group, size, same-school count)
    for group in 1..=GROUP_COUNT {
        let members: Vec<&Person> = people.iter().filter(|p| p.group == group).collect();
        if members.iter().any(|p| p.gender != gender) {
            continue;
        }
        let same_school = members.iter().filter(|p| p.school == school).count();
        candidates.push((group, members.len(), same_school));
    }

    let min_size = candidates.iter().map(|c| c.1).min()?;
    candidates.retain(|c| c.1 == min_size);
    let min_school = candidates.iter().map(|c| c.2).min()?;
    candidates.retain(|c| c.2 == min_school);

    let idx = rand::rng().random_range(0..candidates.len());
    Some(candidates[idx].0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::person::Gender::{Female, Male};

    fn person(id: u64, group: u8, gender: Gender, school: &str) -> Person {
        Person {
            id,
            name: format!("P{id}"),
            class: "8-B".into(),
            school: school.into(),
            gender,
            group,
            avatar_seed: id,
        }
    }

    #[test]
    fn random_group_is_always_in_bounds() {
        for _ in 0..1000 {
            let g = random_group();
            assert!((1..=GROUP_COUNT).contains(&g));
        }
    }

    #[test]
    fn never_mixes_genders_in_a_group() {
        let mut people = Vec::new();
        let mut id = 1;
        for i in 0..165 {
            let gender = if i % 3 == 0 { Female } else { Male };
            let school = ["Southlands", "Convent", "Aloysius"][i % 3];
            let g = pick_group(&people, gender, school).expect("room available");
            people.push(person(id, g, gender, school));
            id += 1;
        }
        for g in 1..=GROUP_COUNT {
            let genders: Vec<Gender> = people.iter().filter(|p| p.group == g).map(|p| p.gender).collect();
            assert!(genders.windows(2).all(|w| w[0] == w[1]), "group {g} mixes genders");
        }
    }

    #[test]
    fn group_sizes_stay_similar_for_one_gender() {
        let mut people = Vec::new();
        for id in 1..=100 {
            let g = pick_group(&people, Male, "Southlands").unwrap();
            people.push(person(id, g, Male, "Southlands"));
        }
        let sizes: Vec<usize> = (1..=GROUP_COUNT)
            .map(|g| people.iter().filter(|p| p.group == g).count())
            .collect();
        assert!(sizes.iter().all(|&s| s == 10), "sizes: {sizes:?}");
    }

    #[test]
    fn same_school_is_spread_across_groups() {
        let mut people = Vec::new();
        for id in 1..=20 {
            let g = pick_group(&people, Female, "Convent").unwrap();
            people.push(person(id, g, Female, "Convent"));
        }
        // 20 girls over 10 groups, all from one school => exactly 2 per group.
        for g in 1..=GROUP_COUNT {
            assert_eq!(people.iter().filter(|p| p.group == g).count(), 2);
        }
    }

    #[test]
    fn returns_none_when_every_group_is_held_by_the_other_gender() {
        let people: Vec<Person> = (1..=GROUP_COUNT)
            .map(|g| person(g as u64, g, Female, "Convent"))
            .collect();
        assert_eq!(pick_group(&people, Male, "Convent"), None);
    }
}
