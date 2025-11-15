use hecs::{Component, Entity, Query, World};
use std::marker::PhantomData;

// 태그가 붙은 컴포넌트를 쿼리하는 trait
pub trait TaggedComponentQuery<Tag> {
    type Component<'a>;
    type QueryType<'a>: Query;

    fn extract_component<'a>(item: <Self::QueryType<'_> as Query>::Item<'a>)
    -> Self::Component<'a>;
}

// 단일 참조에 대한 구현
impl<Tag, T> TaggedComponentQuery<Tag> for &T
where
    for<'a> T: Component + 'a,
    Tag: Component + 'static,
{
    type Component<'a> = &'a T;
    type QueryType<'a> = &'a (Tag, T);

    fn extract_component<'a>(
        item: <Self::QueryType<'a> as Query>::Item<'a>,
    ) -> Self::Component<'a> {
        &item.1
    }
}

// 단일 가변 참조에 대한 구현
impl<Tag, T> TaggedComponentQuery<Tag> for &mut T
where
    for<'a> T: Component + 'a,
    Tag: Component + 'static,
{
    type Component<'a> = &'a mut T;
    type QueryType<'a> = &'a mut (Tag, T);

    fn extract_component<'a>(
        item: <Self::QueryType<'a> as Query>::Item<'a>,
    ) -> Self::Component<'a> {
        &mut item.1
    }
}

// 2개 조합에 대한 재귀적 구현
impl<Tag, A, B> TaggedComponentQuery<Tag> for (A, B)
where
    A: TaggedComponentQuery<Tag>,
    B: TaggedComponentQuery<Tag>,
    Tag: Component + 'static,
{
    type Component<'a> = (A::Component<'a>, B::Component<'a>);
    type QueryType<'a> = (A::QueryType<'a>, B::QueryType<'a>);

    fn extract_component<'a>(
        item: <Self::QueryType<'_> as Query>::Item<'a>,
    ) -> Self::Component<'a> {
        let (item_a, item_b) = item;
        (A::extract_component(item_a), B::extract_component(item_b))
    }
}

// 3개 조합에 대한 재귀적 구현
impl<Tag, A, B, C> TaggedComponentQuery<Tag> for (A, B, C)
where
    A: TaggedComponentQuery<Tag>,
    B: TaggedComponentQuery<Tag>,
    C: TaggedComponentQuery<Tag>,
    Tag: Component + 'static,
{
    type Component<'a> = (A::Component<'a>, B::Component<'a>, C::Component<'a>);
    type QueryType<'a> = (A::QueryType<'a>, B::QueryType<'a>, C::QueryType<'a>);

    fn extract_component<'a>(
        item: <Self::QueryType<'_> as Query>::Item<'a>,
    ) -> Self::Component<'a> {
        let (item_a, item_b, item_c) = item;
        (
            A::extract_component(item_a),
            B::extract_component(item_b),
            C::extract_component(item_c),
        )
    }
}

// 4개 조합에 대한 재귀적 구현
impl<Tag, A, B, C, D> TaggedComponentQuery<Tag> for (A, B, C, D)
where
    A: TaggedComponentQuery<Tag>,
    B: TaggedComponentQuery<Tag>,
    C: TaggedComponentQuery<Tag>,
    D: TaggedComponentQuery<Tag>,
    Tag: Component + 'static,
{
    type Component<'a> = (
        A::Component<'a>,
        B::Component<'a>,
        C::Component<'a>,
        D::Component<'a>,
    );
    type QueryType<'a> = (
        A::QueryType<'a>,
        B::QueryType<'a>,
        C::QueryType<'a>,
        D::QueryType<'a>,
    );

    fn extract_component<'a>(
        item: <Self::QueryType<'_> as Query>::Item<'a>,
    ) -> Self::Component<'a> {
        let (item_a, item_b, item_c, item_d) = item;
        (
            A::extract_component(item_a),
            B::extract_component(item_b),
            C::extract_component(item_c),
            D::extract_component(item_d),
        )
    }
}

// 5개 조합에 대한 재귀적 구현
impl<Tag, A, B, C, D, E> TaggedComponentQuery<Tag> for (A, B, C, D, E)
where
    A: TaggedComponentQuery<Tag>,
    B: TaggedComponentQuery<Tag>,
    C: TaggedComponentQuery<Tag>,
    D: TaggedComponentQuery<Tag>,
    E: TaggedComponentQuery<Tag>,
    Tag: Component + 'static,
{
    type Component<'a> = (
        A::Component<'a>,
        B::Component<'a>,
        C::Component<'a>,
        D::Component<'a>,
        E::Component<'a>,
    );
    type QueryType<'a> = (
        A::QueryType<'a>,
        B::QueryType<'a>,
        C::QueryType<'a>,
        D::QueryType<'a>,
        E::QueryType<'a>,
    );

    fn extract_component<'a>(
        item: <Self::QueryType<'_> as Query>::Item<'a>,
    ) -> Self::Component<'a> {
        let (item_a, item_b, item_c, item_d, item_e) = item;
        (
            A::extract_component(item_a),
            B::extract_component(item_b),
            C::extract_component(item_c),
            D::extract_component(item_d),
            E::extract_component(item_e),
        )
    }
}

// 6개 조합에 대한 재귀적 구현
impl<Tag, A, B, C, D, E, F> TaggedComponentQuery<Tag> for (A, B, C, D, E, F)
where
    A: TaggedComponentQuery<Tag>,
    B: TaggedComponentQuery<Tag>,
    C: TaggedComponentQuery<Tag>,
    D: TaggedComponentQuery<Tag>,
    E: TaggedComponentQuery<Tag>,
    F: TaggedComponentQuery<Tag>,
    Tag: Component + 'static,
{
    type Component<'a> = (
        A::Component<'a>,
        B::Component<'a>,
        C::Component<'a>,
        D::Component<'a>,
        E::Component<'a>,
        F::Component<'a>,
    );
    type QueryType<'a> = (
        A::QueryType<'a>,
        B::QueryType<'a>,
        C::QueryType<'a>,
        D::QueryType<'a>,
        E::QueryType<'a>,
        F::QueryType<'a>,
    );

    fn extract_component<'a>(
        item: <Self::QueryType<'_> as Query>::Item<'a>,
    ) -> Self::Component<'a> {
        let (item_a, item_b, item_c, item_d, item_e, item_f) = item;
        (
            A::extract_component(item_a),
            B::extract_component(item_b),
            C::extract_component(item_c),
            D::extract_component(item_d),
            E::extract_component(item_e),
            F::extract_component(item_f),
        )
    }
}

// 7개 조합에 대한 재귀적 구현
impl<Tag, A, B, C, D, E, F, G> TaggedComponentQuery<Tag> for (A, B, C, D, E, F, G)
where
    A: TaggedComponentQuery<Tag>,
    B: TaggedComponentQuery<Tag>,
    C: TaggedComponentQuery<Tag>,
    D: TaggedComponentQuery<Tag>,
    E: TaggedComponentQuery<Tag>,
    F: TaggedComponentQuery<Tag>,
    G: TaggedComponentQuery<Tag>,
    Tag: Component + 'static,
{
    type Component<'a> = (
        A::Component<'a>,
        B::Component<'a>,
        C::Component<'a>,
        D::Component<'a>,
        E::Component<'a>,
        F::Component<'a>,
        G::Component<'a>,
    );
    type QueryType<'a> = (
        A::QueryType<'a>,
        B::QueryType<'a>,
        C::QueryType<'a>,
        D::QueryType<'a>,
        E::QueryType<'a>,
        F::QueryType<'a>,
        G::QueryType<'a>,
    );

    fn extract_component<'a>(
        item: <Self::QueryType<'_> as Query>::Item<'a>,
    ) -> Self::Component<'a> {
        let (item_a, item_b, item_c, item_d, item_e, item_f, item_g) = item;
        (
            A::extract_component(item_a),
            B::extract_component(item_b),
            C::extract_component(item_c),
            D::extract_component(item_d),
            E::extract_component(item_e),
            F::extract_component(item_f),
            G::extract_component(item_g),
        )
    }
}

// 8개 조합에 대한 재귀적 구현
impl<Tag, A, B, C, D, E, F, G, H> TaggedComponentQuery<Tag> for (A, B, C, D, E, F, G, H)
where
    A: TaggedComponentQuery<Tag>,
    B: TaggedComponentQuery<Tag>,
    C: TaggedComponentQuery<Tag>,
    D: TaggedComponentQuery<Tag>,
    E: TaggedComponentQuery<Tag>,
    F: TaggedComponentQuery<Tag>,
    G: TaggedComponentQuery<Tag>,
    H: TaggedComponentQuery<Tag>,
    Tag: Component + 'static,
{
    type Component<'a> = (
        A::Component<'a>,
        B::Component<'a>,
        C::Component<'a>,
        D::Component<'a>,
        E::Component<'a>,
        F::Component<'a>,
        G::Component<'a>,
        H::Component<'a>,
    );
    type QueryType<'a> = (
        A::QueryType<'a>,
        B::QueryType<'a>,
        C::QueryType<'a>,
        D::QueryType<'a>,
        E::QueryType<'a>,
        F::QueryType<'a>,
        G::QueryType<'a>,
        H::QueryType<'a>,
    );

    fn extract_component<'a>(
        item: <Self::QueryType<'_> as Query>::Item<'a>,
    ) -> Self::Component<'a> {
        let (item_a, item_b, item_c, item_d, item_e, item_f, item_g, item_h) = item;
        (
            A::extract_component(item_a),
            B::extract_component(item_b),
            C::extract_component(item_c),
            D::extract_component(item_d),
            E::extract_component(item_e),
            F::extract_component(item_f),
            G::extract_component(item_g),
            H::extract_component(item_h),
        )
    }
}

// 9개 조합에 대한 재귀적 구현
impl<Tag, A, B, C, D, E, F, G, H, I> TaggedComponentQuery<Tag> for (A, B, C, D, E, F, G, H, I)
where
    A: TaggedComponentQuery<Tag>,
    B: TaggedComponentQuery<Tag>,
    C: TaggedComponentQuery<Tag>,
    D: TaggedComponentQuery<Tag>,
    E: TaggedComponentQuery<Tag>,
    F: TaggedComponentQuery<Tag>,
    G: TaggedComponentQuery<Tag>,
    H: TaggedComponentQuery<Tag>,
    I: TaggedComponentQuery<Tag>,
    Tag: Component + 'static,
{
    type Component<'a> = (
        A::Component<'a>,
        B::Component<'a>,
        C::Component<'a>,
        D::Component<'a>,
        E::Component<'a>,
        F::Component<'a>,
        G::Component<'a>,
        H::Component<'a>,
        I::Component<'a>,
    );
    type QueryType<'a> = (
        A::QueryType<'a>,
        B::QueryType<'a>,
        C::QueryType<'a>,
        D::QueryType<'a>,
        E::QueryType<'a>,
        F::QueryType<'a>,
        G::QueryType<'a>,
        H::QueryType<'a>,
        I::QueryType<'a>,
    );

    fn extract_component<'a>(
        item: <Self::QueryType<'_> as Query>::Item<'a>,
    ) -> Self::Component<'a> {
        let (item_a, item_b, item_c, item_d, item_e, item_f, item_g, item_h, item_i) = item;
        (
            A::extract_component(item_a),
            B::extract_component(item_b),
            C::extract_component(item_c),
            D::extract_component(item_d),
            E::extract_component(item_e),
            F::extract_component(item_f),
            G::extract_component(item_g),
            H::extract_component(item_h),
            I::extract_component(item_i),
        )
    }
}

// 10개 조합에 대한 재귀적 구현
impl<Tag, A, B, C, D, E, F, G, H, I, J> TaggedComponentQuery<Tag> for (A, B, C, D, E, F, G, H, I, J)
where
    A: TaggedComponentQuery<Tag>,
    B: TaggedComponentQuery<Tag>,
    C: TaggedComponentQuery<Tag>,
    D: TaggedComponentQuery<Tag>,
    E: TaggedComponentQuery<Tag>,
    F: TaggedComponentQuery<Tag>,
    G: TaggedComponentQuery<Tag>,
    H: TaggedComponentQuery<Tag>,
    I: TaggedComponentQuery<Tag>,
    J: TaggedComponentQuery<Tag>,
    Tag: Component + 'static,
{
    type Component<'a> = (
        A::Component<'a>,
        B::Component<'a>,
        C::Component<'a>,
        D::Component<'a>,
        E::Component<'a>,
        F::Component<'a>,
        G::Component<'a>,
        H::Component<'a>,
        I::Component<'a>,
        J::Component<'a>,
    );
    type QueryType<'a> = (
        A::QueryType<'a>,
        B::QueryType<'a>,
        C::QueryType<'a>,
        D::QueryType<'a>,
        E::QueryType<'a>,
        F::QueryType<'a>,
        G::QueryType<'a>,
        H::QueryType<'a>,
        I::QueryType<'a>,
        J::QueryType<'a>,
    );

    fn extract_component<'a>(
        item: <Self::QueryType<'_> as Query>::Item<'a>,
    ) -> Self::Component<'a> {
        let (item_a, item_b, item_c, item_d, item_e, item_f, item_g, item_h, item_i, item_j) = item;
        (
            A::extract_component(item_a),
            B::extract_component(item_b),
            C::extract_component(item_c),
            D::extract_component(item_d),
            E::extract_component(item_e),
            F::extract_component(item_f),
            G::extract_component(item_g),
            H::extract_component(item_h),
            I::extract_component(item_i),
            J::extract_component(item_j),
        )
    }
}

// System trait 정의
pub trait System {
    type QueryResult<'a>;

    fn execute<'w, F>(&self, world: &'w World, entity: Entity, func: F)
    where
        F: for<'a> FnOnce(Self::QueryResult<'a>);
}

// Process 시스템 구현
pub struct Process<Tag, Q> {
    _phantom: PhantomData<(Tag, Q)>,
}

impl<Tag, Q> Process<Tag, Q> {
    pub const fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<Tag, Q> System for Process<Tag, Q>
where
    for<'a> Q: TaggedComponentQuery<Tag> + 'a,
    for<'a> Q::QueryType<'a>: Query,
    Tag: Component + 'static,
{
    type QueryResult<'a> = Q::Component<'a>;

    fn execute<'w, F>(&self, world: &'w World, entity: Entity, func: F)
    where
        F: for<'a> FnOnce(Self::QueryResult<'a>),
    {
        let mut query = world
            .query_one::<Q::QueryType<'w>>(entity)
            .expect("invalid entity");
        let result = query.get().expect("invalid entity component!");
        func(Q::extract_component(result))
    }
}

macro_rules! define_tags {
    ( $( $name:ident ),* ) => {
        $(
            #[derive(Debug, Clone, Copy)]
            pub struct $name;

            impl $name {
                #![allow(dead_code)]
                pub fn name() -> &'static str {
                    stringify!($name)
                }
            }
        )*
    }
}

/// 플레이어 태그를 가진 컴포넌트 데이터를 조작합니다.
#[macro_export]
macro_rules! player_execute {
    ($arch: expr, $w: expr, $e: ident, $q: ty, $f: expr) => {
        use crate::component::{
            Player0, Player1, Player2, Player3, Player4, Player5, Player6, Player7, Player8,
            Player9, Process, System,
        };

        match $arch {
            PlayerArchetype::Player0 => {
                let process = Process::<Player0, $q>::new();
                process.execute($w, $e, $f);
            }
            PlayerArchetype::Player1 => {
                let process = Process::<Player1, $q>::new();
                process.execute($w, $e, $f);
            }
            PlayerArchetype::Player2 => {
                let process = Process::<Player2, $q>::new();
                process.execute($w, $e, $f);
            }
            PlayerArchetype::Player3 => {
                let process = Process::<Player3, $q>::new();
                process.execute($w, $e, $f);
            }
            PlayerArchetype::Player4 => {
                let process = Process::<Player4, $q>::new();
                process.execute($w, $e, $f);
            }
            PlayerArchetype::Player5 => {
                let process = Process::<Player5, $q>::new();
                process.execute($w, $e, $f);
            }
            PlayerArchetype::Player6 => {
                let process = Process::<Player6, $q>::new();
                process.execute($w, $e, $f);
            }
            PlayerArchetype::Player7 => {
                let process = Process::<Player7, $q>::new();
                process.execute($w, $e, $f);
            }
            PlayerArchetype::Player8 => {
                let process = Process::<Player8, $q>::new();
                process.execute($w, $e, $f);
            }
            PlayerArchetype::Player9 => {
                let process = Process::<Player9, $q>::new();
                process.execute($w, $e, $f);
            }
        }
    };
}

define_tags!(
    Player0, Player1, Player2, Player3, Player4, Player5, Player6, Player7, Player8, Player9,
    Bullet, Camera, Stage
);

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlayerArchetype {
    Player0 = 0,
    Player1 = 1,
    Player2 = 2,
    Player3 = 3,
    Player4 = 4,
    Player5 = 5,
    Player6 = 6,
    Player7 = 7,
    Player8 = 8,
    Player9 = 9,
}

#[cfg(test)]
mod tests {
    use super::*;

    // 사용 예시
    #[derive(Debug, Clone)]
    pub struct PlayerTag;

    #[derive(Debug, Clone)]
    pub struct EnemyTag;

    #[derive(Debug)]
    pub struct Position(pub f32, pub f32);

    #[derive(Debug)]
    pub struct Health(pub i32);

    #[test]
    fn test_thread_safety() {
        let mut world = World::new();

        // 태그가 붙은 컴포넌트로 엔티티 생성
        let mut player_entities = Vec::new();
        player_entities
            .push(world.spawn(((PlayerTag, Position(10.0, 20.0)), (PlayerTag, Health(100)))));
        player_entities
            .push(world.spawn(((PlayerTag, Position(10.0, 20.0)), (PlayerTag, Health(100)))));
        player_entities
            .push(world.spawn(((PlayerTag, Position(10.0, 20.0)), (PlayerTag, Health(100)))));
        player_entities
            .push(world.spawn(((PlayerTag, Position(10.0, 20.0)), (PlayerTag, Health(100)))));
        player_entities
            .push(world.spawn(((PlayerTag, Position(10.0, 20.0)), (PlayerTag, Health(100)))));
        player_entities
            .push(world.spawn(((PlayerTag, Position(10.0, 20.0)), (PlayerTag, Health(100)))));

        let mut enemy_entities = Vec::new();
        enemy_entities
            .push(world.spawn(((EnemyTag, Position(50.0, 30.0)), (EnemyTag, Health(80)))));
        enemy_entities
            .push(world.spawn(((EnemyTag, Position(50.0, 30.0)), (EnemyTag, Health(80)))));
        enemy_entities
            .push(world.spawn(((EnemyTag, Position(50.0, 30.0)), (EnemyTag, Health(80)))));
        enemy_entities
            .push(world.spawn(((EnemyTag, Position(50.0, 30.0)), (EnemyTag, Health(80)))));
        enemy_entities
            .push(world.spawn(((EnemyTag, Position(50.0, 30.0)), (EnemyTag, Health(80)))));

        rayon::in_place_scope(|scope| {
            scope.spawn(|_| {
                // 플레이어 시스템 - (PlayerTag, Position)과 (PlayerTag, Health)를 쿼리
                let player_process = Process::<PlayerTag, (&mut Position, &mut Health)>::new();
                for entity in player_entities {
                    player_process.execute(&world, entity, |components| {
                        let (pos, health) = components;
                        pos.0 += 10.0;
                        pos.1 -= 10.0;
                        health.0 -= 10;
                    });
                }
            });
            scope.spawn(|_| {
                // 적 시스템 - (EnemyTag, Position)과 (EnemyTag, Health)를 쿼리
                let enemy_process = Process::<EnemyTag, (&mut Position, &mut Health)>::new();
                for entity in enemy_entities {
                    enemy_process.execute(&world, entity, |components| {
                        let (pos, health) = components;
                        pos.0 -= 10.0;
                        pos.1 += 10.0;
                        health.0 -= 10;
                    });
                }
            });
        });
    }
}
