//! # Voting Pallet
//!
//! ## Overview
//!
//! The Voting pallets allows users for quadratic voting for proposals.
//! To join the voting, user needs to get identity. Identity management is implemented
//! using identity pallet.
//!
//! Terms:
//! proposal - the subject on which user vote. It can be anything - it is just an hash for pallet,
//!
//! vote - user defines the vote weight which wants to use for vote
//! proposal_index - each proposal gets index which is incremented with each new proposal, starting
//! from 0,
//!
//! balance lock - balance on user account is locked in order to use it voting. It protects
//! voting system from transfering tokens to different accounts just to do the voting. Account
//! balance lock is square of vote weight.
//!
//! reserved balance - when starting a proposal, user account
//! balance is reserved to protect system against bad proposals(spam). When proposal is identified
//! as a spam user account will be slashed. An proposal is considered as a bad one, when X% of total
//! voting weights is against it. X is configured as SlashThreshold.
//!
//! ## Voting process:
//! #### Proposal start
//! First the proposal needs to be started. It can be done by any user with identity. The proposal
//! is related to a hash of proposal and a voting period in blocks. Proposal starts immediately
//! after call to start_proposal.
//!
//! #### Voting
//! From this point users can start to vote. User can vote and unvote, where the previous vote is
//! deleted. In this case the account balance lock is still present until user call
//! unlock_account_balance. Unlock account balance is a heavy operations, which requires iteration
//! over a vector, so it is not called automatically. When user voted and votes again on the same
//! proposal the previous vote is replaced with the new one. User account balance lock is of the
//! size of the biggest account balance used in voting. Users can vote as long as the proposal is
//! ongoing. Voting is not allowed after the voting period has ended.
//!
//! #### Proposal close
//! Proposal needs to be closed using close_proposal function. It calculates a voting result and
//! emits the event with the result. After this user can unlock his account balance. Proposal is
//! accepted when over 50% votes are aye, in the other case is rejected. Storage data for proposal
//! is removed. To clean storage for votes per user call to unlock_account_balance is needed.
//!
//! Users can vote on multiple proposals at the same time.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub trait IdentityVerifier<AccountId> {
	/// Function that returns whether an account has an identity registered with the identity
	/// provider.
	fn has_identity(who: &AccountId, fields: u64) -> bool;

	/// Whether an account has been deemed "good" by the provider.
	fn has_good_judgement(who: &AccountId) -> bool;
}

/// The non-provider. Imposes no restrictions on account identity.
impl<AccountId> IdentityVerifier<AccountId> for () {
	fn has_identity(_who: &AccountId, _fields: u64) -> bool {
		true
	}

	fn has_good_judgement(_who: &AccountId) -> bool {
		true
	}
}
#[frame_support::pallet]
pub mod pallet {

	use super::*;

	use pallet_identity::IdentityField;
	use scale_info::TypeInfo;

	use frame_support::{
		codec::{Decode, Encode, MaxEncodedLen},
		dispatch::DispatchResult,
		ensure,
		pallet_prelude::*,
		sp_runtime::{
			traits::{CheckedMul, Saturating, StaticLookup, Zero},
			RuntimeDebug,
		},
		traits::{
			Currency, Get, LockIdentifier, LockableCurrency, OnUnbalanced, ReservableCurrency,
			WithdrawReasons,
		},
	};
	use frame_system::pallet_prelude::*;

	/// Proposal index type
	pub type ProposalIndex = u32;

	/// Lock identifier used in lock balance function
	pub const QUADRATIC_VOTE: LockIdentifier = *b"quadvote";
	type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	type NegativeImbalanceOf<T> = <<T as Config>::Currency as Currency<
		<T as frame_system::Config>::AccountId,
	>>::NegativeImbalance;

	type AccountIdLookupOf<T> = <<T as frame_system::Config>::Lookup as StaticLookup>::Source;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Type to access the Balances Pallet.
		type Currency: Currency<Self::AccountId>
			+ ReservableCurrency<Self::AccountId>
			+ LockableCurrency<Self::AccountId>;

		/// Handler for slashing proposal deposit.
		type Slash: OnUnbalanced<NegativeImbalanceOf<Self>>;

		/// Identiy verifier of voting member
		type IdentityVerifier: IdentityVerifier<Self::AccountId>;
		/// Maximal count of proposals any time -  recommended u32 max
		#[pallet::constant]
		type MaxProposals: Get<ProposalIndex>;
		/// Maximal count of votes per user
		#[pallet::constant]
		type MaxVotes: Get<u32>;
		/// Reserved balance on user account for proposal
		#[pallet::constant]
		type ProposalDeposit: Get<BalanceOf<Self>>;
		/// Slash threshold for proposal to be considered as spam
		#[pallet::constant]
		type SlashThreshold: Get<u32>;
	}

	/// Proposal status represents the status in which is proposal after start to its close.
	/// Proposal can be closed with approved or rejected status based on votes weight
	#[derive(Encode, MaxEncodedLen, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
	pub enum ProposalStatus {
		/// Proposal is started and ongoing
		ProposalStarted,
		/// Proposal is closed with approved status
		ProposalApproved,
		/// Proposal is closed with reject status
		ProposalRejected,
	}

	/// Default trait implementation
	impl Default for ProposalStatus {
		fn default() -> Self {
			ProposalStatus::ProposalStarted
		}
	}

	/// Info regarding an ongoing proposal.
	#[derive(Encode, MaxEncodedLen, Decode, Clone, PartialEq, Eq, RuntimeDebug, TypeInfo)]
	#[scale_info(skip_type_params(T))]
	pub struct Proposal<T: Config> {
		/// Block number after which the voting on this referendum will end.
		pub end: T::BlockNumber,
		/// Hash of the proposal being voted on.
		pub proposal: T::Hash,
		/// The quadretic vote weight for aye votes.
		pub aye: BalanceOf<T>,
		/// The quadratic vote weight for nay votes.
		pub nay: BalanceOf<T>,
		/// Status of voting
		pub status: ProposalStatus,
	}

	impl<T: Config> Proposal<T> {
		/// Calculates the proposal result status
		fn finalize(&mut self) -> ProposalStatus {
			self.status = match self.aye.cmp(&self.nay) {
				core::cmp::Ordering::Greater => ProposalStatus::ProposalApproved,
				_ => ProposalStatus::ProposalRejected,
			};
			self.status.clone()
		}
		/// Creates new proposal
		fn new(hash: T::Hash, end: T::BlockNumber) -> Proposal<T> {
			Proposal {
				end,
				proposal: hash,
				aye: Zero::zero(),
				nay: Zero::zero(),
				status: Default::default(),
			}
		}
		/// Adds user votes to proposal
		fn add_vote(&mut self, vote: &Vote<T>) {
			match *vote {
				Vote::Aye(v) => {
					self.aye = self.aye.saturating_add(v);
				},
				Vote::Nay(v) => {
					self.nay = self.nay.saturating_add(v);
				},
			}
		}
		/// Removes user votes to proposal
		fn remove_vote(&mut self, vote: &Vote<T>) {
			match *vote {
				Vote::Aye(v) => {
					self.aye = self.aye.saturating_sub(v);
				},
				Vote::Nay(v) => {
					self.nay = self.nay.saturating_sub(v);
				},
			}
		}

		/// Checks if proposal is good
		fn check_spam(&self) -> bool {
			//if most votes against proposal - proposal is kind of spam
			let total = self.aye.saturating_add(self.nay);
			total.saturating_mul(T::SlashThreshold::get().into()) <=
				self.nay.saturating_mul(100u32.into())
		}
	}

	/// Enum represents user vote. Vote contains its weight
	#[derive(
		Encode, MaxEncodedLen, Decode, Clone, PartialEq, Eq, TypeInfo, RuntimeDebugNoBound,
	)]
	#[scale_info(skip_type_params(T))]
	pub enum Vote<T: Config> {
		/// Aye vote
		Aye(BalanceOf<T>),
		/// Nay vote
		Nay(BalanceOf<T>),
	}

	impl<T: Config> Vote<T> {
		/// Gets vote veight
		fn get_weight(&self) -> BalanceOf<T> {
			match *self {
				Self::Aye(b) | Self::Nay(b) => b,
			}
		}

		/// Calculates lock balance of vote
		fn get_balance_lock(&self) -> Option<BalanceOf<T>> {
			let v = self.get_weight();
			v.checked_mul(&v)
		}
	}

	/// User votes per proposal, used to manage the account balance locks
	#[derive(Encode, MaxEncodedLen, Decode, Clone, PartialEq, Eq, TypeInfo, RuntimeDebug)]
	#[scale_info(skip_type_params(T))]
	pub struct ProposalVotes<T: Config> {
		/// Keeps vote per proposal. It is linked to user by the map in storage
		pub votes: BoundedVec<(ProposalIndex, Vote<T>), T::MaxVotes>,
	}

	/// Default trait implementation
	impl<T: Config> Default for ProposalVotes<T> {
		fn default() -> Self {
			ProposalVotes { votes: Default::default() }
		}
	}

	impl<T: Config> ProposalVotes<T> {
		/// Get the bigest balance lock from user votes
		fn get_balance_lock(&self) -> BalanceOf<T> {
			self.votes
				.iter()
				.map(|i| i.1.get_balance_lock().unwrap_or_default())
				.fold(0u32.into(), |a, i| a.max(i))
		}
	}
	// The pallet's runtime storage items.
	/// The count of proposal which has been done.
	#[pallet::storage]
	#[pallet::getter(fn proposal_count)]
	pub type ProposalCount<T> = StorageValue<_, ProposalIndex, ValueQuery>;

	// Twox64Concat used on safely incremented internal index on Accounts is used Blake2_128Concat
	/// Deposit locked on proposal owner
	#[pallet::storage]
	#[pallet::getter(fn proposal_owner)]
	pub type DepositOf<T: Config> =
		StorageMap<_, Twox64Concat, ProposalIndex, (T::AccountId, BalanceOf<T>)>;

	/// Proposal status and detailed data
	#[pallet::storage]
	#[pallet::getter(fn proposal_info)]
	pub type ProposalOf<T: Config> = StorageMap<_, Twox64Concat, ProposalIndex, Proposal<T>>;

	/// Votes locked balance of each user
	#[pallet::storage]
	#[pallet::getter(fn user_votes)]
	pub type VotingOf<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, ProposalVotes<T>, ValueQuery>;

	/// Genesis block setup
	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		_phantom: sp_std::marker::PhantomData<T>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig { _phantom: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			ProposalCount::<T>::put(0 as ProposalIndex);
		}
	}

	// Pallets use events to inform users when important changes are made.
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Proposal started to be voted
		/// parameters [account, proposal_index]
		/// account - account which started proposal
		/// proposal_index - handle to operate on proposal
		ProposalStarted { account: T::AccountId, proposal_index: ProposalIndex },
		/// Proposal closed with voting status which can be accepted or rejected
		/// parameters [proposal_index, status]
		/// proposal_index - handle to operate on proposal
		/// status - result of voting on proposal: ProposalAccepted or ProposalRejected
		ProposalClosed { proposal_index: ProposalIndex, status: ProposalStatus },
		/// Voted on proposal with given hash
		/// parameters [account, proposal_index, vote]
		/// account - account which voted on proposal
		/// proposal_index - handle to operate on proposal
		/// vote - aye or nay with its weight
		Voted { account: T::AccountId, proposal_index: ProposalIndex, vote: Vote<T> },
		/// Removed vote on given proposal
		/// parameters [account, proposal_index, vote]
		/// account - account which unvoted proposal
		/// proposal_index - handle to operate on proposal
		/// vote - aye or nay with its weight
		UnVoted { account: T::AccountId, proposal_index: ProposalIndex, vote: Vote<T> },
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// The account's identity does not have display field and website field.
		WithoutIdentityDisplayAndWebsite,
		/// The account's identity has no good judgement.
		WithoutGoodIdentityJudgement,
		///Set Voting time is to low
		VotingPeriodLow,
		///Number of proposals reached max limit
		ProposalsMaxCount,
		///Invalid proposal Index
		ProposalIndexInvalid,
		///Proposal still under voting
		ProposalUnderVoting,
		///Proposal still under voting
		ProposalClosedForVoting,
		///User account free balance to low in order to vote
		BalanceToLow,
		///User used max votes pool
		VotesMaxCount,
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Start proposal. Voting is valid until current block + voting period blocks specified in
		/// voting_period Only users which have identity can start proposal.
		///
		/// Prameters:
		/// origin - origin of the call
		/// proposal - hash of the proposal
		/// voting_period - duration in number of blocks
		///
		/// Returns: DispatchResult
		///
		/// Events:
		/// ProposalStarted - emitted on success
		#[pallet::call_index(0)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn start_proposal(
			origin: OriginFor<T>,
			proposal: T::Hash,
			voting_period: T::BlockNumber,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::has_identity(&who)?;
			ensure!(voting_period > T::BlockNumber::zero(), Error::<T>::VotingPeriodLow);
			let current_block = <frame_system::Pallet<T>>::block_number();
			let voting_end = current_block.saturating_add(voting_period);

			let index = Self::proposal_count();
			ensure!(index < T::MaxProposals::get(), Error::<T>::ProposalsMaxCount);
			T::Currency::reserve(&who, T::ProposalDeposit::get())?;

			let status: Proposal<T> = Proposal::new(proposal, voting_end);
			ProposalOf::<T>::insert(index, status);
			ProposalCount::<T>::put(index + 1);
			//Stored proposal owner with deposit
			DepositOf::<T>::insert(index, (who.clone(), T::ProposalDeposit::get()));

			Self::deposit_event(Event::<T>::ProposalStarted {
				account: who,
				proposal_index: index,
			});
			Ok(())
		}

		/// Close proposal it can be closed after voting has been finished
		/// Only users which have identity can close proposal.
		///
		/// Prameters:
		/// origin - origin of the call
		/// proposal_index - handle of the proposal
		///
		/// Returns: DispatchResult
		///
		/// Events:
		/// ProposalClosed - emitted on success
		#[pallet::call_index(1)]
		#[pallet::weight(10_000 + T::DbWeight::get().writes(1).ref_time())]
		pub fn close_proposal(
			origin: OriginFor<T>,
			proposal_index: ProposalIndex,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::has_identity(&who)?;

			let mut proposal =
				ProposalOf::<T>::get(proposal_index).ok_or(Error::<T>::ProposalIndexInvalid)?;

			let current_block = <frame_system::Pallet<T>>::block_number();
			ensure!(proposal.end < current_block, Error::<T>::ProposalUnderVoting);

			//proposal ended
			if let Some((who, deposit)) = DepositOf::<T>::take(proposal_index) {
				if proposal.check_spam() {
					//Slash user if bad proposal
					T::Slash::on_unbalanced(
						T::Currency::slash_reserved(&who, T::ProposalDeposit::get()).0,
					);
				} else {
					// unreserve deposit
					T::Currency::unreserve(&who, deposit);
				}
			}
			//removing proposal data from storage, only will be kept user locks related to
			// proposals
			ProposalOf::<T>::remove(proposal_index);

			Self::deposit_event(Event::<T>::ProposalClosed {
				proposal_index,
				status: proposal.finalize(),
			});
			Ok(())
		}

		/// Vote on proposal
		/// Only users which have identity can vote on proposal.
		///
		/// Prameters:
		/// origin - origin of the call
		/// proposal_index - handle of the proposal
		/// vote - aye or nay
		///
		/// Returns: DispatchResult
		///
		/// Events:
		/// UnVoted - emitted when user votes again on the same proposal. Previous vote is unvoted
		/// Voted - emmited when user voted
		#[pallet::call_index(2)]
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1).ref_time())]
		pub fn vote(
			origin: OriginFor<T>,
			proposal_index: ProposalIndex,
			vote: Vote<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::has_identity(&who)?;
			let balance = vote.get_balance_lock().ok_or(Error::<T>::BalanceToLow)?;
			ensure!(balance <= T::Currency::free_balance(&who), Error::<T>::BalanceToLow);
			Self::is_proposal_ongoing(&proposal_index)?;
			ProposalOf::<T>::try_mutate(proposal_index, |info| -> DispatchResult {
				if let Some(proposal_info) = info {
					VotingOf::<T>::try_mutate(&who, |votes| -> DispatchResult {
						match votes.votes.binary_search_by_key(&proposal_index, |(i, _)| *i) {
							Ok(idx) => {
								//votes again on the same proposal
								if let Some(v) = votes.votes.get(idx) {
									proposal_info.remove_vote(&v.1);
									Self::deposit_event(Event::<T>::UnVoted {
										account: who.clone(),
										proposal_index,
										vote: v.1.clone(),
									});
									proposal_info.add_vote(&vote);
									votes.votes[idx].1 = vote.clone();
									Self::deposit_event(Event::<T>::Voted {
										account: who.clone(),
										proposal_index,
										vote,
									});
								}
							},
							Err(idx) => {
								//voting for the first time
								proposal_info.add_vote(&vote);
								votes
									.votes
									.try_insert(idx, (proposal_index, vote.clone()))
									.map_err(|_| Error::<T>::VotesMaxCount)?;
								Self::deposit_event(Event::<T>::Voted {
									account: who.clone(),
									proposal_index,
									vote,
								});
							},
						}
						Ok(())
					})?;
				} else {
					//it never should go here
					return Err(Error::<T>::ProposalIndexInvalid.into())
				}
				Ok(())
			})?;
			//keep the lock for the biggest amount of balance
			T::Currency::extend_lock(QUADRATIC_VOTE, &who, balance, WithdrawReasons::TRANSFER);
			Ok(())
		}

		/// Unvote proposal it needs to be in started state
		/// Only users which have identity can unvote proposal.
		/// This function does not unlock balance, unlock account balance needs to be called
		///
		/// Prameters:
		/// origin - origin of the call
		/// proposal_index - handle of the proposal
		/// vote - aye or nay
		///
		/// Returns: DispatchResult
		///
		/// Events:
		/// UnVoted - emitted when user unvoted proposal
		#[pallet::call_index(3)]
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1).ref_time())]
		pub fn unvote(origin: OriginFor<T>, proposal_index: ProposalIndex) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::has_identity(&who)?;
			Self::is_proposal_ongoing(&proposal_index)?;

			ProposalOf::<T>::try_mutate(proposal_index, |info| -> DispatchResult {
				if let Some(proposal_info) = info {
					VotingOf::<T>::try_mutate(&who, |votes| -> DispatchResult {
						if let Ok(idx) =
							votes.votes.binary_search_by_key(&proposal_index, |(i, _)| *i)
						{
							//votes found
							match votes.votes.get(idx) {
								Some((_, v)) => {
									proposal_info.remove_vote(v);
									Self::deposit_event(Event::<T>::UnVoted {
										account: who.clone(),
										proposal_index,
										vote: v.clone(),
									});
								},
								_ => {
									//it never should go here
									return Err(Error::<T>::ProposalIndexInvalid.into())
								},
							}
							votes.votes.remove(idx);
						} else {
							//vote is not present
							return Err(Error::<T>::ProposalIndexInvalid.into())
						}

						Ok(())
					})?;
				} else {
					return Err(Error::<T>::ProposalIndexInvalid.into())
				}
				Ok(())
			})?;
			Ok(())
		}

		/// Unlock account balance if possible on user request
		/// Only users which have identity can call unlock
		///
		/// Prameters:
		/// origin - origin of the call
		/// account - account to be unlocked
		/// proposal_index - handle of the proposal
		///
		/// Returns: DispatchResult
		#[pallet::call_index(4)]
		#[pallet::weight(10_000 + T::DbWeight::get().reads_writes(1,1).ref_time())]
		pub fn unlock_account_balance(
			origin: OriginFor<T>,
			account: AccountIdLookupOf<T>,
			proposal_index: ProposalIndex,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::has_identity(&who)?;
			let account = T::Lookup::lookup(account)?;
			let current_block = <frame_system::Pallet<T>>::block_number();
			let end = ProposalOf::<T>::get(proposal_index)
				.map_or(T::BlockNumber::default(), |proposal| proposal.end);
			let mut empty = false;
			VotingOf::<T>::try_mutate(&who, |votes| -> DispatchResult {
				if let Ok(idx) = votes.votes.binary_search_by_key(&proposal_index, |(i, _)| *i) {
					//cleanup account votes
					if end < current_block {
						//proposal ended, can cleanup votes
						votes.votes.remove(idx);
						empty = votes.votes.is_empty();
					}
				}
				//try unlock, maybe user did unvote in meantime
				Self::try_unlock(&account, votes)?;
				Ok(())
			})?;
			//check if we can clean user account
			if empty {
				VotingOf::<T>::remove(&who);
			}
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Proposal is still ongoing
		fn is_proposal_ongoing(proposal_index: &ProposalIndex) -> DispatchResult {
			let current_block = <frame_system::Pallet<T>>::block_number();
			let proposal =
				ProposalOf::<T>::get(proposal_index).ok_or(Error::<T>::ProposalIndexInvalid)?;
			ensure!(proposal.end >= current_block, Error::<T>::ProposalClosedForVoting);
			Ok(())
		}
		/// Try unlock balance on user account
		fn try_unlock(who: &T::AccountId, votes: &ProposalVotes<T>) -> DispatchResult {
			let balance = votes.get_balance_lock();
			if balance.is_zero() {
				T::Currency::remove_lock(QUADRATIC_VOTE, who);
			} else {
				T::Currency::set_lock(QUADRATIC_VOTE, who, balance, WithdrawReasons::TRANSFER);
			}
			Ok(())
		}
		/// Check if user has identity
		fn has_identity(who: &T::AccountId) -> DispatchResult {
			const IDENTITY_FIELD_DISPLAY: u64 = IdentityField::Display as u64;
			const IDENTITY_FIELD_WEB: u64 = IdentityField::Web as u64;

			let judgement = |who: &T::AccountId| -> DispatchResult {
				ensure!(
					T::IdentityVerifier::has_identity(
						who,
						IDENTITY_FIELD_DISPLAY | IDENTITY_FIELD_WEB
					),
					Error::<T>::WithoutIdentityDisplayAndWebsite
				);
				ensure!(
					T::IdentityVerifier::has_good_judgement(who),
					Error::<T>::WithoutGoodIdentityJudgement
				);
				Ok(())
			};

			judgement(who)
		}
	}
}
