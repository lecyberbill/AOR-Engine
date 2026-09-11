// [WFGY] Zone: SAFE | λ: 0.2 | Fallbacks: 0 | Action: Modular Component & Event-Driven Gameplay Architecture
#![allow(dead_code)]
use std::any::{Any, TypeId};
use std::collections::HashMap;

/// Identifiant unique d'une entité du moteur
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EntityId(pub usize);

/// Trait universel d'un composant de jeu/moteur
pub trait Component: 'static + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: 'static + Send + Sync> Component for T {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Événement de gameplay ou du moteur
#[derive(Clone, Debug, PartialEq)]
pub enum EngineEvent {
    EntityCreated(EntityId),
    EntityDestroyed(EntityId),
    CollisionEnter { entity_a: EntityId, entity_b: EntityId },
    ExplosionTriggered { position: glam::Vec3, radius: f32, force: f32 },
    Custom { name: String, payload: String },
}

/// Bus d'événements et conteneur de composants modulaire
pub struct WorldEcs {
    pub next_id: usize,
    pub entities: Vec<EntityId>,
    pub components: HashMap<TypeId, HashMap<EntityId, Box<dyn Component>>>,
    pub event_queue: Vec<EngineEvent>,
}

impl Default for WorldEcs {
    fn default() -> Self {
        Self {
            next_id: 0,
            entities: Vec::new(),
            components: HashMap::new(),
            event_queue: Vec::new(),
        }
    }
}

impl WorldEcs {
    pub fn new() -> Self {
        Self::default()
    }

    /// Crée une nouvelle entité dans le monde
    pub fn spawn_entity(&mut self) -> EntityId {
        let id = EntityId(self.next_id);
        self.next_id += 1;
        self.entities.push(id);
        self.event_queue.push(EngineEvent::EntityCreated(id));
        id
    }

    /// Attache un composant modulaire à une entité
    pub fn add_component<T: Component>(&mut self, entity: EntityId, component: T) {
        let type_id = TypeId::of::<T>();
        self.components
            .entry(type_id)
            .or_default()
            .insert(entity, Box::new(component));
    }

    /// Récupère une référence vers le composant d'une entité
    pub fn get_component<T: 'static + Component>(&self, entity: EntityId) -> Option<&T> {
        let type_id = TypeId::of::<T>();
        let comp_map = self.components.get(&type_id)?;
        let boxed = comp_map.get(&entity)?;
        (**boxed).as_any().downcast_ref::<T>()
    }

    /// Récupère une référence mutable vers le composant d'une entité
    pub fn get_component_mut<T: 'static + Component>(&mut self, entity: EntityId) -> Option<&mut T> {
        let type_id = TypeId::of::<T>();
        let comp_map = self.components.get_mut(&type_id)?;
        let boxed = comp_map.get_mut(&entity)?;
        (**boxed).as_any_mut().downcast_mut::<T>()
    }


    /// Émet un événement dans le bus
    pub fn emit_event(&mut self, event: EngineEvent) {
        self.event_queue.push(event);
    }

    /// Consomme tous les événements de la frame
    pub fn drain_events(&mut self) -> Vec<EngineEvent> {
        self.event_queue.drain(..).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Health(pub f32);
    struct Velocity(pub glam::Vec3);

    #[test]
    fn test_modular_ecs_components_and_event_bus() {
        let mut world = WorldEcs::new();
        let player = world.spawn_entity();

        world.add_component(player, Health(100.0));
        world.add_component(player, Velocity(glam::Vec3::new(0.0, 5.0, 0.0)));

        if let Some(h) = world.get_component_mut::<Health>(player) {
            h.0 -= 25.0;
        }

        assert_eq!(world.get_component::<Health>(player).unwrap().0, 75.0);
        assert_eq!(world.get_component::<Velocity>(player).unwrap().0.y, 5.0);

        let events = world.drain_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0], EngineEvent::EntityCreated(player));
    }
}
