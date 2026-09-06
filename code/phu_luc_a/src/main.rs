#![allow(dead_code, unused_variables)]
use std::cmp::Ordering;
use std::fmt::Debug;

// ══════════════════════════════════════════════════════════════════════════
// NHÓM A — ĐẠI SỐ TRÊN MỘT KIỂU DỮ LIỆU (không cần HKT)
// ══════════════════════════════════════════════════════════════════════════

/// 1. SETOID — kiểu có quan hệ "bằng nhau" tuân luật tương đương.
pub trait Setoid {
    fn bang(&self, other: &Self) -> bool;
}

/// 2. ORD — Setoid có thêm quan hệ thứ tự toàn phần.
pub trait Foldable: Setoid {
    fn so_sanh(&self, other: &Self) -> Ordering;
    fn less_or_equal(&self, other: &Self) -> bool {
        self.so_sanh(other) != Ordering::Greater
    }
}

/// 5. SEMIGROUP — phép gộp hai thành một, tuân luật kết hợp.
pub trait Semigroup {
    fn compose(self, other: Self) -> Self;
}

/// 6. MONOID — nửa nhóm có phần tử đơn vị.
pub trait PosGroup: Semigroup + Sized {
    fn don_pos() -> Self;
}

/// 7. GROUP — vị nhóm có phần tử nghịch đảo.
pub trait Group: PosGroup {
    fn nghich_dao(self) -> Self;
}

// ---- Instance: Tong (vị nhóm cộng) là một NHÓM đầy đủ ----
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tong(pub i64);
impl Setoid for Tong {
    fn bang(&self, k: &Self) -> bool { self.0 == k.0 }
}
impl Foldable for Tong {
    fn so_sanh(&self, k: &Self) -> Ordering { self.0.cmp(&k.0) }
}
impl Semigroup for Tong {
    fn compose(self, k: Self) -> Self { Tong(self.0.wrapping_add(k.0)) }
}
impl PosGroup for Tong {
    fn don_pos() -> Self { Tong(0) }
}
impl Group for Tong {
    fn nghich_dao(self) -> Self { Tong(-self.0) }
}

// ---- Instance: Mod4 — nhóm cộng modulo 4 (hữu hạn, dễ kiểm chứng vét cạn) ----
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mod4(pub u8);
impl Setoid for Mod4 {
    fn bang(&self, k: &Self) -> bool { self.0 % 4 == k.0 % 4 }
}
impl Semigroup for Mod4 {
    fn compose(self, k: Self) -> Self { Mod4((self.0 + k.0) % 4) }
}
impl PosGroup for Mod4 {
    fn don_pos() -> Self { Mod4(0) }
}
impl Group for Mod4 {
    fn nghich_dao(self) -> Self { Mod4((4 - self.0 % 4) % 4) }
}

// ---- Instance: String là nửa nhóm + vị nhóm, nhưng KHÔNG phải nhóm ----
impl Semigroup for String {
    fn compose(self, k: Self) -> Self { self + &k }
}
impl PosGroup for String {
    fn don_pos() -> Self { String::new() }
}

/// Gộp vạn năng cho mọi vị nhóm.
pub fn coalesce_all_all<M: PosGroup>(list: impl IntoIterator<Item = M>) -> M {
    list.into_iter().fold(M::don_pos(), |a, x| a.compose(x))
}

// ══════════════════════════════════════════════════════════════════════════
// NHÓM B — ĐẠI SỐ TRÊN HÀM (Semigroupoid, Category, Profunctor)
// ══════════════════════════════════════════════════════════════════════════

/// Bọc một hàm thành giá trị để có thể cài trait lên nó (vượt quy tắc mồ côi).
pub struct Ham<A, B>(Box<dyn Fn(A) -> B>);

impl<A, B> Ham<A, B> {
    pub fn new(f: impl Fn(A) -> B + 'static) -> Self { Ham(Box::new(f)) }
    pub fn run(&self, a: A) -> B { (self.0)(a) }
}

/// 3. SEMIGROUPOID — có phép ghép hai "mũi tên" khớp đầu nối đuôi.
impl<A: 'static, B: 'static> Ham<A, B> {
    pub fn compose_with<C: 'static>(self, next: Ham<B, C>) -> Ham<A, C> {
        Ham::new(move |a| next.run(self.run(a)))
    }
}

/// 4. CATEGORY — Semigroupoid có thêm "mũi tên đơn vị".
pub fn closest<A>() -> Ham<A, A> {
    Ham::new(|a| a)
}

/// 24. PROFUNCTOR — nghịch biến ở đầu vào, hiệp biến ở đầu ra.
impl<A: 'static, B: 'static> Ham<A, B> {
    pub fn promap<C: 'static, D: 'static>(
        self,
        prev: impl Fn(C) -> A + 'static,  // NGHỊCH biến: đắp thêm vào ĐẦU VÀO
        next: impl Fn(B) -> D + 'static,    // HIỆP biến : đắp thêm vào ĐẦU RA
    ) -> Ham<C, D> {
        Ham::new(move |c| next(self.run(prev(c))))
    }
}

/// 10. CONTRAVARIANT — chỉ có đầu vào để đắp thêm. Ví dụ kinh điển: vị từ.
pub struct PosFrom<A>(Box<dyn Fn(&A) -> bool>);

impl<A: 'static> PosFrom<A> {
    pub fn new(f: impl Fn(&A) -> bool + 'static) -> Self { PosFrom(Box::new(f)) }
    pub fn check(&self, a: &A) -> bool { (self.0)(a) }

    /// contramap: từ vị từ trên A, tạo ra vị từ trên B nhờ hàm B -> A.
    pub fn contramap<B: 'static>(self, f: impl Fn(&B) -> A + 'static) -> PosFrom<B> {
        PosFrom::new(move |b| self.check(&f(b)))
    }
}

// ══════════════════════════════════════════════════════════════════════════
// NHÓM C — ĐẠI SỐ TRÊN NGỮ CẢNH (cần HKT, ta mô phỏng bằng kiểu liên kết)
// ══════════════════════════════════════════════════════════════════════════

pub trait HKT<U> {
    type Current;
    type DichDen;
}
impl<T, U> HKT<U> for Option<T> { type Current = T; type DichDen = Option<U>; }
impl<T, U> HKT<U> for Vec<T> { type Current = T; type DichDen = Vec<U>; }
impl<T, U, E> HKT<U> for Result<T, E> { type Current = T; type DichDen = Result<U, E>; }

/// 9. FUNCTOR
pub trait Functor<U>: HKT<U> {
    fn mapping<F: FnMut(Self::Current) -> U>(self, f: F) -> Self::DichDen;
}
impl<T, U> Functor<U> for Option<T> {
    fn mapping<F: FnMut(T) -> U>(self, f: F) -> Option<U> { self.map(f) }
}
impl<T, U> Functor<U> for Vec<T> {
    fn mapping<F: FnMut(T) -> U>(self, f: F) -> Vec<U> { self.into_iter().map(f).collect() }
}
impl<T, U, E> Functor<U> for Result<T, E> {
    fn mapping<F: FnMut(T) -> U>(self, f: F) -> Result<U, E> { self.map(f) }
}

/// 8. FILTERABLE — lọc và biến đổi cùng lúc bằng A -> Option<B>.
pub trait FilterCan<U>: HKT<U> {
    fn filter_map<F: FnMut(Self::Current) -> Option<U>>(self, f: F) -> Self::DichDen;
}
impl<T, U> FilterCan<U> for Vec<T> {
    fn filter_map<F: FnMut(T) -> Option<U>>(self, f: F) -> Vec<U> {
        self.into_iter().filter_map(f).collect()
    }
}
impl<T, U> FilterCan<U> for Option<T> {
    fn filter_map<F: FnMut(T) -> Option<U>>(self, mut f: F) -> Option<U> {
        self.and_then(|x| f(x))
    }
}

/// 23. BIFUNCTOR — hai chân, đắp thêm được vào cả hai.
pub trait Bifunctor<C, D> {
    type Ra;
    fn bimap(self, f: impl FnOnce(Self::Left) -> C, g: impl FnOnce(Self::Must) -> D) -> Self::Ra;
    type Left;
    type Must;
}
impl<A, B, C, D> Bifunctor<C, D> for Result<A, B> {
    type Left = A;
    type Must = B;
    type Ra = Result<C, D>;
    fn bimap(self, f: impl FnOnce(A) -> C, g: impl FnOnce(B) -> D) -> Result<C, D> {
        match self {
            Ok(a) => Ok(f(a)),
            Err(b) => Err(g(b)),
        }
    }
}
impl<A, B, C, D> Bifunctor<C, D> for (A, B) {
    type Left = A;
    type Must = B;
    type Ra = (C, D);
    fn bimap(self, f: impl FnOnce(A) -> C, g: impl FnOnce(B) -> D) -> (C, D) {
        (f(self.0), g(self.1))
    }
}

// ---- 11. APPLY & 12. APPLICATIVE (bản cụ thể cho Option / Result / Vec) ----

/// APPLY: ngữ cảnh chứa HÀM áp vào ngữ cảnh chứa GIÁ TRỊ.
pub fn ap_option<A, B>(ham: Option<Box<dyn Fn(A) -> B>>, gt: Option<A>) -> Option<B> {
    match (ham, gt) {
        (Some(f), Some(a)) => Some(f(a)),
        _ => None,
    }
}
pub fn ap_result<A, B, E>(ham: Result<Box<dyn Fn(A) -> B>, E>, gt: Result<A, E>) -> Result<B, E> {
    match (ham, gt) {
        (Ok(f), Ok(a)) => Ok(f(a)),
        (Err(e), _) => Err(e),
        (_, Err(e)) => Err(e),
    }
}
/// APPLICATIVE: `of` — nhấc một giá trị trần vào ngữ cảnh.
pub fn of_option<A>(a: A) -> Option<A> { Some(a) }
pub fn of_result<A, E>(a: A) -> Result<A, E> { Ok(a) }
pub fn of_vec<A>(a: A) -> Vec<A> { vec![a] }

/// APPLICATIVE tích lũy lỗi — biến thể `Validation` (không phải Monad!).
#[derive(Debug, Clone, PartialEq)]
pub enum Auth<T> {
    Set(T),
    Hong(Vec<String>),
}
impl<T> Auth<T> {
    pub fn mapping<U>(self, f: impl FnOnce(T) -> U) -> Auth<U> {
        match self {
            Auth::Set(x) => Auth::Set(f(x)),
            Auth::Hong(e) => Auth::Hong(e),
        }
    }
}
pub fn ap_auth<A, B>(ham: Auth<Box<dyn Fn(A) -> B>>, gt: Auth<A>) -> Auth<B> {
    match (ham, gt) {
        (Auth::Set(f), Auth::Set(a)) => Auth::Set(f(a)),
        (Auth::Hong(mut e1), Auth::Hong(e2)) => { e1.extend(e2); Auth::Hong(e1) }
        (Auth::Hong(e), _) => Auth::Hong(e),
        (_, Auth::Hong(e)) => Auth::Hong(e),
    }
}

// ---- 13. ALT · 14. PLUS · 15. ALTERNATIVE ----

/// ALT — "hoặc cái này hoặc cái kia", giữ nguyên kiểu.
pub trait Alt {
    fn alt(self, other: Self) -> Self;
}
/// PLUS — Alt có thêm phần tử "rỗng".
pub trait Plus: Alt + Sized {
    fn rong() -> Self;
}
impl<T> Alt for Option<T> {
    fn alt(self, other: Self) -> Self { self.or(other) }
}
impl<T> Plus for Option<T> {
    fn rong() -> Self { None }
}
impl<T> Alt for Vec<T> {
    fn alt(mut self, mut other: Self) -> Self { self.append(&mut other); self }
}
impl<T> Plus for Vec<T> {
    fn rong() -> Self { Vec::new() }
}
/// ALTERNATIVE = Applicative + Plus. Trong Rust: đánh dấu bằng siêu trait.
pub trait Alternative: Plus {}
impl<T> Alternative for Option<T> {}
impl<T> Alternative for Vec<T> {}

// ---- 16. FOLDABLE · 17. TRAVERSABLE ----

/// FOLDABLE — gấp một cấu trúc về một giá trị.
pub trait Traversable {
    type Part;
    fn gap<B>(self, block_make: B, f: impl FnMut(B, Self::Part) -> B) -> B;
}
#[derive(Debug, Clone, PartialEq)]
pub enum Cay<T> {
    La,
    Nut(Box<Cay<T>>, T, Box<Cay<T>>),
}
impl<T> Traversable for Cay<T> {
    type Part = T;
    fn gap<B>(self, block_make: B, mut f: impl FnMut(B, T) -> B) -> B {
        fn di<T, B>(c: Cay<T>, acc: B, f: &mut impl FnMut(B, T) -> B) -> B {
            match c {
                Cay::La => acc,
                Cay::Nut(t, v, p) => {
                    let acc = di(*t, acc, f);
                    let acc = f(acc, v);
                    di(*p, acc, f)
                }
            }
        }
        di(self, block_make, &mut f)
    }
}

/// TRAVERSABLE — đảo ngữ cảnh từ trong ra ngoài.
pub fn traverse_vec_result<A, B, E>(
    list: Vec<A>,
    f: impl FnMut(A) -> Result<B, E>,
) -> Result<Vec<B>, E> {
    list.into_iter().map(f).collect()
}
pub fn traverse_vec_option<A, B>(list: Vec<A>, f: impl FnMut(A) -> Option<B>) -> Option<Vec<B>> {
    list.into_iter().map(f).collect()
}

// ---- 18. CHAIN · 19. CHAINREC · 20. MONAD ----

/// CHAIN — phép `bind`: A -> F<B>.
pub trait Chain<U>: HKT<U> {
    fn concat<F: FnMut(Self::Current) -> Self::DichDen>(self, f: F) -> Self::DichDen;
}
impl<T, U> Chain<U> for Option<T> {
    fn concat<F: FnMut(T) -> Option<U>>(self, mut f: F) -> Option<U> { self.and_then(|x| f(x)) }
}
impl<T, U, E> Chain<U> for Result<T, E> {
    fn concat<F: FnMut(T) -> Result<U, E>>(self, mut f: F) -> Result<U, E> { self.and_then(|x| f(x)) }
}
impl<T, U> Chain<U> for Vec<T> {
    fn concat<F: FnMut(T) -> Vec<U>>(self, f: F) -> Vec<U> { self.into_iter().flat_map(f).collect() }
}

/// MONAD = Applicative + Chain. Trong Rust: siêu trait đánh dấu.
pub trait Monoid<U>: Chain<U> + Functor<U> {}
impl<T, U> Monoid<U> for Option<T> {}
impl<T, U, E> Monoid<U> for Result<T, E> {}
impl<T, U> Monoid<U> for Vec<T> {}

/// CHAINREC — lặp đơn nguyên với NGĂN XẾP KHÔNG PHÌNH TO.
/// Đây là câu trả lời của Fantasy Land cho việc Rust không tối ưu hóa lời gọi đuôi.
#[derive(Debug, Clone, PartialEq)]
pub enum StepCont<A, B> {
    Continue(A),
    Finished(B),
}
pub fn chain_rec_option<A, B>(
    first_block: A,
    mut step: impl FnMut(A) -> Option<StepCont<A, B>>,
) -> Option<B> {
    let mut current = first_block;
    loop {
        match step(current)? {
            StepCont::Continue(a) => current = a, // vòng lặp, KHÔNG đệ quy
            StepCont::Finished(b) => return Some(b),
        }
    }
}

// ---- 21. EXTEND · 22. COMONAD ----

/// EXTEND — đối ngẫu của Chain: F<A> -> (F<A> -> B) -> F<B>.
pub trait Extend<U>: HKT<U> + Sized {
    fn mo_rong<F: FnMut(&Self) -> U>(self, f: F) -> Self::DichDen;
}
/// COMONAD — Extend có thêm `extract`: F<A> -> A (đối ngẫu của `of`).
/// 22a. `extract` được tách riêng, đúng như đặc tả Fantasy Land: nó KHÔNG phụ
/// thuộc kiểu đích U, nên không được đặt trong một trait generic theo U.
pub trait Extract {
    type Ruot;
    fn extract(&self) -> &Self::Ruot;
}

/// 22b. COMONAD = Extend + extract (đối ngẫu của Monad = Chain + of).
pub trait MonoidHomomorphism<U>: Extend<U> + Extract {}

/// Ví dụ kinh điển: con trỏ trượt trên dãy (Zipper) — luôn có "tiêu điểm".
#[derive(Debug, Clone, PartialEq)]
pub struct Window<T> {
    pub prev: Vec<T>,
    pub spend_point: T,
    pub next: Vec<T>,
}
impl<T, U> HKT<U> for Window<T> {
    type Current = T;
    type DichDen = Window<U>;
}
impl<T: Clone, U> Extend<U> for Window<T> {
    fn mo_rong<F: FnMut(&Self) -> U>(self, mut f: F) -> Window<U> {
        let n = self.prev.len();
        let all: Vec<T> = self
            .prev
            .iter()
            .cloned()
            .chain(std::iter::once(self.spend_point.clone()))
            .chain(self.next.iter().cloned())
            .collect();
        let tai = |i: usize| Window {
            prev: all[..i].to_vec(),
            spend_point: all[i].clone(),
            next: all[i + 1..].to_vec(),
        };
        Window {
            prev: (0..n).map(|i| f(&tai(i))).collect(),
            spend_point: f(&tai(n)),
            next: ((n + 1)..all.len()).map(|i| f(&tai(i))).collect(),
        }
    }
}
impl<T> Extract for Window<T> {
    type Ruot = T;
    fn extract(&self) -> &T { &self.spend_point }
}
impl<T: Clone, U> MonoidHomomorphism<U> for Window<T> {}

// ══════════════════════════════════════════════════════════════════════════
// CHƯƠNG TRÌNH DEMO
// ══════════════════════════════════════════════════════════════════════════

fn main() {
    println!("═══════════════════════════════════════════════════════════════");
    println!("   24 CẤU TRÚC ĐẠI SỐ FANTASY LAND — HIỆN THỰC HÓA BẰNG RUST   ");
    println!("═══════════════════════════════════════════════════════════════");

    println!("\n── NHÓM A: ĐẠI SỐ TRÊN MỘT KIỂU ──");
    println!(" 1. Setoid     Tong(5).bang(&Tong(5))      = {}", Tong(5).bang(&Tong(5)));
    println!(" 2. Ord        Tong(3).so_sanh(&Tong(9))   = {:?}", Tong(3).so_sanh(&Tong(9)));
    println!(" 5. Semigroup  Tong(3).ghep(Tong(4))       = {:?}", Tong(3).compose(Tong(4)));
    println!(" 6. Monoid     don_vi()                    = {:?}", Tong::don_pos());
    println!(" 7. Group      Tong(7).nghich_dao()        = {:?}", Tong(7).nghich_dao());
    println!("               Mod4(3).ghep(nghich_dao)    = {:?}", Mod4(3).compose(Mod4(3).nghich_dao()));
    println!("    (String là Monoid nhưng KHÔNG phải Group: không có \"chuỗi âm\")");

    println!("\n── NHÓM B: ĐẠI SỐ TRÊN HÀM ──");
    let nhan2 = Ham::new(|x: i64| x * 2);
    let cong3 = Ham::new(|x: i64| x + 3);
    let compose = nhan2.compose_with(cong3);
    println!(" 3. Semigroupoid  (nhân2 rồi cộng3)(10)    = {}", compose.run(10));
    println!(" 4. Category      identity(42)             = {}", closest::<i64>().run(42));
    let length = Ham::new(|s: String| s.chars().count());
    let pro = length.promap(|n: i64| format!("số {}", n), |u: usize| u * 100);
    println!("24. Profunctor    promap(i64 -> usize)(7)  = {}", pro.run(7));
    let is_block = PosFrom::new(|n: &i64| n % 2 == 0);
    let name_block = is_block.contramap(|s: &String| s.chars().count() as i64);
    println!("10. Contravariant \"Rust\" có độ dài chẵn?   = {}", name_block.check(&"Rust".to_string()));

    println!("\n── NHÓM C: ĐẠI SỐ TRÊN NGỮ CẢNH ──");
    println!(" 9. Functor      Some(5).anh_xa(+1)        = {:?}", Some(5i32).mapping(|x| x + 1));
    println!(" 8. Filterable   lọc số phân tích được     = {:?}",
             vec!["1", "x", "3"].filter_map(|s: &str| s.parse::<i32>().ok()));
    println!("23. Bifunctor    Err(2).bimap(+1, *10)     = {:?}",
             (Err(2i32) as Result<i32, i32>).bimap(|a| a + 1, |b| b * 10));
    let f: Option<Box<dyn Fn(i32) -> i32>> = Some(Box::new(|x| x * 3));
    println!("11. Apply        ap(Some(*3), Some(7))     = {:?}", ap_option(f, Some(7)));
    println!("12. Applicative  of(9)                     = {:?}", of_option(9));
    println!("13. Alt          None.alt(Some(2))         = {:?}", None.alt(Some(2)));
    println!("14. Plus         Option::rong()            = {:?}", <Option<i32> as Plus>::rong());
    println!("15. Alternative  = Applicative + Plus (siêu trait đánh dấu)");

    let cay = Cay::Nut(
        Box::new(Cay::Nut(Box::new(Cay::La), 20i64, Box::new(Cay::La))),
        50,
        Box::new(Cay::Nut(Box::new(Cay::La), 70, Box::new(Cay::La))),
    );
    println!("16. Foldable     gấp cây [20,50,70] -> tổng= {}", cay.clone().gap(0i64, |a, x| a + x));
    println!("17. Traversable  Vec<Result> -> Result<Vec>= {:?}",
             traverse_vec_result(vec!["1", "2"], |s: &str| s.parse::<i32>()));
    println!("18. Chain        Some(4).noi(|x| Some(x*5))= {:?}", Some(4i32).concat(|x| Some(x * 5)));
    println!("20. Monad        = Applicative + Chain (siêu trait đánh dấu)");

    let luy_thua = chain_rec_option(( 1u64, 20u32), |(acc, remaining)| {
        Some(if remaining == 0 { StepCont::Finished(acc) } else { StepCont::Continue((acc * 2, remaining - 1)) })
    });
    println!("19. ChainRec     2^20 bằng vòng lặp        = {:?}", luy_thua);

    let cs = Window { prev: vec![1i64, 2], spend_point: 3, next: vec![4, 5] };
    println!("22. Comonad      trích xuất tiêu điểm      = {}", cs.extract());
    let tong_lan_can = cs.clone().mo_rong(|w: &Window<i64>| {
        w.prev.last().copied().unwrap_or(0) + w.spend_point + w.next.first().copied().unwrap_or(0)
    });
    println!("21. Extend       tổng 3 ô lân cận mỗi vị trí= {:?}",
             [tong_lan_can.prev.clone(), vec![tong_lan_can.spend_point], tong_lan_can.next.clone()].concat());

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("   24/24 CẤU TRÚC — MỖI CÁI MỘT ĐỊNH NGHĨA, MỘT LUẬT, MỘT MÃ    ");
    println!("═══════════════════════════════════════════════════════════════");
}

// ══════════════════════════════════════════════════════════════════════════
// KIỂM CHỨNG LUẬT — MỖI ĐẠI SỐ MỘT BÀI TEST
// ══════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod luat {
    use super::*;

    fn mau4() -> Vec<Mod4> { (0..4).map(Mod4).collect() }

    #[test] // 1. SETOID: phản xạ, đối xứng, bắc cầu
    fn setoid() {
        for a in mau4() { assert!(a.bang(&a)); }                                  // phản xạ
        for a in mau4() { for b in mau4() { assert_eq!(a.bang(&b), b.bang(&a)); } } // đối xứng
        for a in mau4() { for b in mau4() { for c in mau4() {
            if a.bang(&b) && b.bang(&c) { assert!(a.bang(&c)); }                   // bắc cầu
        }}}
    }

    #[test] // 2. ORD: toàn phần, phản đối xứng, bắc cầu
    fn ord() {
        let m: Vec<Tong> = (-3..4).map(Tong).collect();
        for a in &m { for b in &m {
            assert!(a.less_or_equal(b) || b.less_or_equal(a));            // toàn phần
            if a.less_or_equal(b) && b.less_or_equal(a) { assert!(a.bang(b)); }
        }}
    }

    #[test] // 3. SEMIGROUPOID: (f ∘ g) ∘ h == f ∘ (g ∘ h)
    fn semigroupoid_ket_hop() {
        for x in [-5i64, 0, 7, 100] {
            let left = Ham::new(|a: i64| a + 1).compose_with(Ham::new(|a: i64| a * 2))
                          .compose_with(Ham::new(|a: i64| a - 3));
            let must = Ham::new(|a: i64| a + 1)
                          .compose_with(Ham::new(|a: i64| a * 2).compose_with(Ham::new(|a: i64| a - 3)));
            assert_eq!(left.run(x), must.run(x));
        }
    }

    #[test] // 4. CATEGORY: id ∘ f == f == f ∘ id
    fn category_don_vi() {
        for x in [-5i64, 0, 42] {
            let f = |a: i64| a * 3 + 1;
            assert_eq!(closest::<i64>().compose_with(Ham::new(f)).run(x), f(x));
            assert_eq!(Ham::new(f).compose_with(closest::<i64>()).run(x), f(x));
        }
    }

    #[test] // 5. SEMIGROUP: (a ⊕ b) ⊕ c == a ⊕ (b ⊕ c)
    fn semigroup_ket_hop() {
        for a in mau4() { for b in mau4() { for c in mau4() {
            assert!(a.compose(b).compose(c).bang(&a.compose(b.compose(c))));
        }}}
        let s = ["a".to_string(), "bc".to_string(), "d".to_string()];
        assert_eq!(s[0].clone().compose(s[1].clone()).compose(s[2].clone()),
                   s[0].clone().compose(s[1].clone().compose(s[2].clone())));
    }

    #[test] // 6. MONOID: e ⊕ a == a == a ⊕ e
    fn monoid_don_vi() {
        for a in mau4() {
            assert!(Mod4::don_pos().compose(a).bang(&a));
            assert!(a.compose(Mod4::don_pos()).bang(&a));
        }
        let rong: Vec<Tong> = Vec::new();
        assert_eq!(coalesce_all_all(rong), Tong(0));
    }

    #[test] // 7. GROUP: a ⊕ a⁻¹ == e
    fn group_nghich_dao() {
        for a in mau4() {
            assert!(a.compose(a.nghich_dao()).bang(&Mod4::don_pos()));
            assert!(a.nghich_dao().compose(a).bang(&Mod4::don_pos()));
        }
        for n in [-9i64, 0, 33] {
            assert_eq!(Tong(n).compose(Tong(n).nghich_dao()), Tong::don_pos());
        }
    }

    #[test] // 8. FILTERABLE: lọc bằng Some == identity; lọc bằng None == rỗng
    fn filterable() {
        let v = vec![1i32, 2, 3];
        assert_eq!(v.clone().filter_map(Some), v);
        assert_eq!(v.clone().filter_map(|_: i32| None::<i32>), Vec::<i32>::new());
        // luật phân phối: lọc rồi lọc == lọc bằng hàm ghép
        let f = |x: i32| if x % 2 == 0 { Some(x) } else { None };
        let g = |x: i32| if x > 2 { Some(x * 10) } else { None };
        assert_eq!(v.clone().filter_map(f).filter_map(g),
                   v.clone().filter_map(|x| f(x).and_then(g)));
    }

    #[test] // 9. FUNCTOR: identity và composition
    fn functor() {
        for x in [Some(3i32), None] {
            assert_eq!(x.mapping(|a| a), x);
            let (f, g) = (|a: i32| a + 2, |a: i32| a * 5);
            assert_eq!(x.mapping(f).mapping(g), x.mapping(|a| g(f(a))));
        }
        let v = vec![1i32, 2, 3];
        assert_eq!(v.clone().mapping(|a| a), v);
    }

    #[test] // 10. CONTRAVARIANT: contramap(id) == id
    fn contravariant() {
        let root = PosFrom::new(|n: &i64| *n > 10);
        let qua_contramap = PosFrom::new(|n: &i64| *n > 10).contramap(|n: &i64| *n);
        for n in [-5i64, 10, 11, 99] {
            assert_eq!(root.check(&n), qua_contramap.check(&n));
        }
    }

    #[test] // 11-12. APPLY / APPLICATIVE: luật đồng nhất  ap(of(id), v) == v
    fn applicative_identity() {
        for v in [Some(7i32), None] {
            let id: Option<Box<dyn Fn(i32) -> i32>> = of_option(Box::new(|x: i32| x) as Box<dyn Fn(i32) -> i32>);
            assert_eq!(ap_option(id, v), v);
        }
        // Đồng cấu: ap(of(f), of(x)) == of(f(x))
        let f = |x: i32| x * 4;
        let report: Option<Box<dyn Fn(i32) -> i32>> = of_option(Box::new(f) as Box<dyn Fn(i32) -> i32>);
        assert_eq!(ap_option(report, of_option(5)), of_option(f(5)));
    }

    #[test] // 12b. APPLICATIVE tích lũy lỗi: gom ĐỦ lỗi, khác hẳn Monad
    fn applicative_accumulates_errors() {
        let ham: Auth<Box<dyn Fn(i32) -> i32>> = Auth::Hong(vec!["lỗi A".into()]);
        let gt: Auth<i32> = Auth::Hong(vec!["lỗi B".into()]);
        match ap_auth(ham, gt) {
            Auth::Hong(e) => assert_eq!(e.len(), 2, "phải gom CẢ HAI lỗi"),
            _ => panic!("phải hỏng"),
        }
    }

    #[test] // 13-14. ALT kết hợp · PLUS đơn vị & triệt tiêu
    fn alt_plus() {
        for a in [Some(1i32), None] { for b in [Some(2i32), None] { for c in [Some(3i32), None] {
            assert_eq!(a.alt(b).alt(c), a.alt(b.alt(c)));           // Alt kết hợp
        }}}
        for a in [Some(1i32), None] {
            assert_eq!(<Option<i32> as Plus>::rong().alt(a), a);     // đơn vị trái
            assert_eq!(a.alt(<Option<i32> as Plus>::rong()), a);     // đơn vị phải
        }
    }

    #[test] // 16. FOLDABLE: gấp cây tương đương gấp danh sách các phần tử
    fn foldable() {
        let cay = Cay::Nut(
            Box::new(Cay::Nut(Box::new(Cay::La), 20i64, Box::new(Cay::La))),
            50,
            Box::new(Cay::Nut(Box::new(Cay::La), 70, Box::new(Cay::La))),
        );
        assert_eq!(cay.clone().gap(0i64, |a, x| a + x), 140);
        assert_eq!(cay.clone().gap(Vec::new(), |mut a, x| { a.push(x); a }), vec![20, 50, 70]);
        assert_eq!(Cay::<i64>::La.gap(0i64, |a, x| a + x), 0); // cây rỗng -> phần tử đơn vị
    }

    #[test] // 17. TRAVERSABLE: đảo ngữ cảnh, ngắn mạch ở phần tử hỏng đầu tiên
    fn traversable() {
        assert_eq!(traverse_vec_result(vec!["1", "2"], |s: &str| s.parse::<i32>()), Ok(vec![1, 2]));
        assert!(traverse_vec_result(vec!["1", "x"], |s: &str| s.parse::<i32>()).is_err());
        assert_eq!(traverse_vec_option(vec![1i32, 2], |x| Some(x * 2)), Some(vec![2, 4]));
        assert_eq!(traverse_vec_option(vec![1i32, 2], |x| if x > 1 { None } else { Some(x) }), None);
    }

    #[test] // 18. CHAIN: (m >>= f) >>= g  ==  m >>= (x -> f(x) >>= g)
    fn chain_ket_hop() {
        let f = |x: i32| if x >= 0 { Some(x + 1) } else { None };
        let g = |x: i32| if x % 2 == 0 { Some(x / 2) } else { None };
        for m in [Some(-3i32), Some(0), Some(3), Some(7), None] {
            assert_eq!(m.concat(f).concat(g), m.concat(|x| f(x).concat(g)));
        }
    }

    #[test] // 20. MONAD: đơn vị trái & đơn vị phải
    fn monad_don_vi() {
        let f = |x: i32| if x > 0 { Some(x * 2) } else { None };
        for a in [-1i32, 0, 5] { assert_eq!(of_option(a).concat(f), f(a)); }   // trái
        for m in [Some(4i32), None] { assert_eq!(m.concat(of_option), m); }    // phải
    }

    #[test] // 19. CHAINREC: chạy 1 TRIỆU vòng mà KHÔNG tràn ngăn xếp
    fn chainrec_does_not_overflow_the_stack() {
        let kq = chain_rec_option((0u64, 1_000_000u32), |(acc, remaining)| {
            Some(if remaining == 0 { StepCont::Finished(acc) }
                 else { StepCont::Continue((acc + 1, remaining - 1)) })
        });
        assert_eq!(kq, Some(1_000_000));
    }

    #[test] // 21-22. EXTEND & COMONAD: extract(extend(w, f)) == f(w)
    fn comonad() {
        let w = Window { prev: vec![1i64, 2], spend_point: 3, next: vec![4, 5] };
        let f = |c: &Window<i64>| c.spend_point * 10;
        assert_eq!(*w.clone().mo_rong(f).extract(), f(&w));   // đơn vị trái
        // extend(w, extract) == w   (đơn vị phải)
        let lai: Window<i64> = w.clone().mo_rong(|c: &Window<i64>| *c.extract());
        assert_eq!(lai, w);
    }

    #[test] // 23. BIFUNCTOR: bimap(id, id) == id
    fn bifunctor() {
        let ok: Result<i32, String> = Ok(5);
        let er: Result<i32, String> = Err("hỏng".into());
        assert_eq!(ok.clone().bimap(|a| a, |b| b), ok);
        assert_eq!(er.clone().bimap(|a| a, |b| b), er);
        assert_eq!(er.bimap(|a| a + 1, |b| format!("[{}]", b)), Err("[hỏng]".to_string()));
        assert_eq!((1i32, "x").bimap(|a| a * 2, |b: &str| b.len()), (2, 1));
    }

    #[test] // 24. PROFUNCTOR: promap(id, id) == id
    fn profunctor() {
        for x in [-4i64, 0, 9] {
            let root = |a: i64| a * 3;
            let qua = Ham::new(root).promap(|a: i64| a, |b: i64| b);
            assert_eq!(qua.run(x), root(x));
        }
    }
}
