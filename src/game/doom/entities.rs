// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0/None | Action: Interactive DOOM Entities: Explosive Barrels, Sliding Doors, Imps and Pickups
use glam::Vec3;

#[derive(Clone, Debug, PartialEq)]
pub enum EntityKind {
    ExplosiveBarrel { health: f32, exploded: bool },
    ToxicPool { damage_tick: f32 },
    Door { open_amount: f32, is_opening: bool, speed: f32 },
    HealthBonus { amount: i32, collected: bool },
    ArmorBonus { amount: i32, collected: bool },
    DemonImp { health: f32, speed: f32, attack_cooldown: f32 },
}

#[derive(Clone, Debug)]
pub struct DoomEntity {
    pub id: usize,
    pub name: String,
    pub position: Vec3,
    pub radius: f32,
    pub kind: EntityKind,
    pub node_idx: Option<usize>,
}

impl DoomEntity {
    pub fn new_barrel(id: usize, pos: Vec3, node_idx: usize) -> Self {
        Self {
            id,
            name: format!("Toxic_Explosive_Barrel_{}", id),
            position: pos,
            radius: 0.65,
            kind: EntityKind::ExplosiveBarrel {
                health: 20.0,
                exploded: false,
            },
            node_idx: Some(node_idx),
        }
    }

    pub fn new_door(id: usize, pos: Vec3, node_idx: usize) -> Self {
        Self {
            id,
            name: format!("Hydraulic_Blast_Door_{}", id),
            position: pos,
            radius: 2.2,
            kind: EntityKind::Door {
                open_amount: 0.0,
                is_opening: false,
                speed: 1.8,
            },
            node_idx: Some(node_idx),
        }
    }

    pub fn new_imp(id: usize, pos: Vec3, node_idx: usize) -> Self {
        Self {
            id,
            name: format!("Demon_Imp_{}", id),
            position: pos,
            radius: 0.75,
            kind: EntityKind::DemonImp {
                health: 60.0,
                speed: 3.2,
                attack_cooldown: 0.0,
            },
            node_idx: Some(node_idx),
        }
    }

    pub fn update(&mut self, dt: f32, player_pos: Vec3) -> Option<EntityEvent> {
        match &mut self.kind {
            EntityKind::Door { open_amount, is_opening, speed } => {
                let dist_to_player = (player_pos - self.position).length();
                if dist_to_player < 4.0 {
                    *is_opening = true;
                } else if dist_to_player > 6.0 {
                    *is_opening = false;
                }

                if *is_opening {
                    *open_amount = (*open_amount + *speed * dt).min(1.0);
                } else {
                    *open_amount = (*open_amount - *speed * dt).max(0.0);
                }
                None
            }
            EntityKind::DemonImp { health, speed, attack_cooldown } => {
                if *health <= 0.0 {
                    return None;
                }
                *attack_cooldown = (*attack_cooldown - dt).max(0.0);

                let to_player = player_pos - self.position;
                let dist = to_player.length();

                if dist > 2.0 && dist < 18.0 {
                    let dir = Vec3::new(to_player.x, 0.0, to_player.z).normalize_or_zero();
                    self.position += dir * *speed * dt;
                } else if dist <= 2.2 && *attack_cooldown <= 0.0 {
                    *attack_cooldown = 1.2;
                    return Some(EntityEvent::ImpAttack { damage: 15 });
                }
                None
            }
            _ => None,
        }
    }

    pub fn apply_damage(&mut self, damage: f32) -> Option<EntityEvent> {
        match &mut self.kind {
            EntityKind::ExplosiveBarrel { health, exploded } => {
                if *exploded {
                    return None;
                }
                *health -= damage;
                if *health <= 0.0 {
                    *exploded = true;
                    return Some(EntityEvent::BarrelExploded {
                        position: self.position,
                        radius: 5.5,
                        damage: 85.0,
                    });
                }
                None
            }
            EntityKind::DemonImp { health, .. } => {
                if *health <= 0.0 {
                    return None;
                }
                *health -= damage;
                if *health <= 0.0 {
                    return Some(EntityEvent::ImpDied { id: self.id });
                }
                None
            }
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub enum EntityEvent {
    BarrelExploded { position: Vec3, radius: f32, damage: f32 },
    ImpAttack { damage: i32 },
    ImpDied { id: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_barrel_explosion() {
        let mut barrel = DoomEntity::new_barrel(1, Vec3::new(2.0, 0.0, 2.0), 0);
        let ev1 = barrel.apply_damage(10.0);
        assert!(ev1.is_none());

        let ev2 = barrel.apply_damage(15.0);
        assert!(matches!(ev2, Some(EntityEvent::BarrelExploded { .. })));
    }

    #[test]
    fn test_door_automatic_opening() {
        let mut door = DoomEntity::new_door(2, Vec3::ZERO, 0);
        door.update(0.5, Vec3::new(2.0, 0.0, 0.0)); // Joueur proche (2m)
        if let EntityKind::Door { open_amount, .. } = door.kind {
            assert!(open_amount > 0.0);
        } else {
            panic!("Wrong entity kind");
        }
    }
}
