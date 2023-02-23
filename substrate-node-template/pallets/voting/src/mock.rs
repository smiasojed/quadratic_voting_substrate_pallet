use crate as pallet_voting;
use frame_support::{
	assert_ok, ord_parameter_types, parameter_types,
	traits::{ConstU16, ConstU64, EitherOfDiverse, GenesisBuild, OnFinalize, OnInitialize},
	BoundedVec,
};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, Hash, IdentityLookup},
};
type AccountId = u64;

use pallet_identity::{Data, IdentityInfo, Judgement};
use pallet_voting::*;

use frame_system::{EnsureRoot, EnsureSignedBy};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
type Balance = u128;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system,
		Balances: pallet_balances,
		Voting: pallet_voting,
		Identity: pallet_identity,
	}
);

impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstU16<42>;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
	pub const MaxLocks: u32 = 10;
	pub const MotionDuration: u64 = 3;
	pub const MaxProposals: u32 = 16;
	#[derive(PartialEq)]
	pub const MaxVotes: u32 = 4;
	pub const ProposalDeposit: u128 = 10;
	pub const MaxRegistrars: u32 = 20;
	pub const SlashThreshold: u32 = 90;

}
impl pallet_balances::Config for Test {
	type Balance = u128;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxLocks = MaxLocks;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
}

ord_parameter_types! {
	pub const One: u64 = 1;
	pub const Two: u64 = 2;
}

type EnsureOneOrRoot = EitherOfDiverse<EnsureRoot<AccountId>, EnsureSignedBy<One, AccountId>>;
type EnsureTwoOrRoot = EitherOfDiverse<EnsureRoot<AccountId>, EnsureSignedBy<Two, AccountId>>;
impl pallet_identity::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type BasicDeposit = ();
	type FieldDeposit = ();
	type SubAccountDeposit = ();
	type MaxSubAccounts = ();
	type MaxAdditionalFields = ();
	type MaxRegistrars = MaxRegistrars;
	type Slashed = ();
	type RegistrarOrigin = EnsureOneOrRoot;
	type ForceOrigin = EnsureTwoOrRoot;
	type WeightInfo = ();
}

pub struct VotingIdentityVerifier;
impl pallet_voting::IdentityVerifier<AccountId> for VotingIdentityVerifier {
	fn has_identity(who: &AccountId, fields: u64) -> bool {
		Identity::has_identity(who, fields)
	}

	fn has_good_judgement(who: &AccountId) -> bool {
		if let Some(judgements) =
			Identity::identity(who).map(|registration| registration.judgements)
		{
			judgements
				.iter()
				.any(|(_, j)| matches!(j, Judgement::KnownGood | Judgement::Reasonable))
		} else {
			false
		}
	}
}

impl pallet_voting::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type IdentityVerifier = VotingIdentityVerifier;
	type MaxProposals = MaxProposals;
	type MaxVotes = MaxVotes;
	type ProposalDeposit = ProposalDeposit;
	type Slash = ();
	type SlashThreshold = SlashThreshold;
}

// Prepare tesing environment
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
	// Configure balances in genesis block
	pallet_balances::GenesisConfig::<Test> {
		balances: vec![(1, 5), (2, 20), (3, 30), (4, 40), (5, 50), (6, 60)],
	}
	.assimilate_storage(&mut t)
	.unwrap();
	pallet_voting::GenesisConfig::<Test>::default()
		.assimilate_storage(&mut t)
		.unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| {
		assert_ok!(Identity::add_registrar(RuntimeOrigin::signed(1), 1));
		// Set user identities used in tests
		let info = IdentityInfo {
			additional: BoundedVec::default(),
			display: Data::Raw(b"name".to_vec().try_into().unwrap()),
			legal: Data::default(),
			web: Data::Raw(b"website".to_vec().try_into().unwrap()),
			riot: Data::default(),
			email: Data::default(),
			pgp_fingerprint: None,
			image: Data::default(),
			twitter: Data::default(),
		};
		assert_ok!(Identity::set_identity(RuntimeOrigin::signed(1), Box::new(info.clone())));
		assert_ok!(Identity::provide_judgement(
			RuntimeOrigin::signed(1),
			0,
			1,
			Judgement::KnownGood,
			BlakeTwo256::hash_of(&info)
		));
		assert_ok!(Identity::set_identity(RuntimeOrigin::signed(2), Box::new(info.clone())));
		assert_ok!(Identity::provide_judgement(
			RuntimeOrigin::signed(1),
			0,
			2,
			Judgement::Erroneous,
			BlakeTwo256::hash_of(&info)
		));
		assert_ok!(Identity::set_identity(RuntimeOrigin::signed(3), Box::new(info.clone())));
		assert_ok!(Identity::provide_judgement(
			RuntimeOrigin::signed(1),
			0,
			3,
			Judgement::KnownGood,
			BlakeTwo256::hash_of(&info)
		));
		assert_ok!(Identity::set_identity(RuntimeOrigin::signed(4), Box::new(info.clone())));
		assert_ok!(Identity::provide_judgement(
			RuntimeOrigin::signed(1),
			0,
			4,
			Judgement::KnownGood,
			BlakeTwo256::hash_of(&info)
		));
		assert_ok!(Identity::set_identity(RuntimeOrigin::signed(5), Box::new(info.clone())));
		assert_ok!(Identity::provide_judgement(
			RuntimeOrigin::signed(1),
			0,
			5,
			Judgement::KnownGood,
			BlakeTwo256::hash_of(&info)
		));
		assert_ok!(Identity::set_identity(RuntimeOrigin::signed(6), Box::new(info.clone())));
		assert_ok!(Identity::set_identity(RuntimeOrigin::signed(8), Box::new(info.clone())));
		assert_ok!(Identity::provide_judgement(
			RuntimeOrigin::signed(1),
			0,
			8,
			Judgement::KnownGood,
			BlakeTwo256::hash_of(&info)
		));
		assert_ok!(Identity::set_identity(RuntimeOrigin::signed(9), Box::new(info.clone())));
		assert_ok!(Identity::provide_judgement(
			RuntimeOrigin::signed(1),
			0,
			9,
			Judgement::KnownGood,
			BlakeTwo256::hash_of(&info)
		));

		System::set_block_number(1);
	});
	ext
}

// Get storage proposal data
pub fn storage_proposal(index: ProposalIndex) -> Option<Proposal<Test>> {
	ProposalOf::<Test>::get(index)
}

// Get proposal index from storage
pub fn storage_proposal_counter() -> ProposalIndex {
	ProposalCount::<Test>::get()
}

// Get reserved balance for user
pub fn storage_deposit(
	index: ProposalIndex,
) -> Option<(<Test as frame_system::Config>::AccountId, Balance)> {
	DepositOf::<Test>::get(index)
}

// Get user votes per proposal
pub fn storage_user_votes(
	account: <Test as frame_system::Config>::AccountId,
) -> ProposalVotes<Test> {
	VotingOf::<Test>::get(account)
}

// Create proposal hash
pub fn new_proposal() -> H256 {
	let proposal: String = "my new proposal".to_owned();
	<Test as frame_system::Config>::Hashing::hash(proposal.as_bytes())
}

// Create vector with user votes per proposal
pub fn new_votes_of(
	v: Vec<(ProposalIndex, Vote<Test>)>,
) -> BoundedVec<(ProposalIndex, Vote<Test>), MaxVotes> {
	BoundedVec::truncate_from(v)
}

// Create lock struct with amount
pub fn the_lock(amount: u128) -> pallet_balances::BalanceLock<u128> {
	pallet_balances::BalanceLock {
		id: QUADRATIC_VOTE,
		amount,
		reasons: pallet_balances::Reasons::Misc,
	}
}

// Go to block
pub fn run_to_block(n: u64) {
	while System::block_number() < n {
		if System::block_number() > 1 {
			System::on_finalize(System::block_number());
		}
		System::set_block_number(System::block_number() + 1);
		System::on_initialize(System::block_number());
	}
}
