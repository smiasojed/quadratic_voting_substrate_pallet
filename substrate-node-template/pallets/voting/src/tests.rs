use crate::{mock::*, Error, Event, Proposal, ProposalIndex, ProposalStatus, ProposalVotes, Vote};
use frame_support::{assert_noop, assert_ok};

// Start proposal tests success and error
// Start proposal insufficient account balance error
#[test]
fn balance_error_for_start_proposal() {
	new_test_ext().execute_with(|| {
		System::set_block_number(0);
		assert_noop!(
			Voting::start_proposal(RuntimeOrigin::signed(1), new_proposal(), 3),
			pallet_balances::Error::<Test, ()>::InsufficientBalance
		);
	});
}

// Start proposal identity error WithoutIdentityDisplayAndWebsite
#[test]
fn identity_error_for_start_proposal() {
	new_test_ext().execute_with(|| {
		System::set_block_number(0);
		assert_noop!(
			Voting::start_proposal(RuntimeOrigin::signed(7), new_proposal(), 3),
			Error::<Test>::WithoutIdentityDisplayAndWebsite,
		);
	});
}

// Start proposal identity error WithoutGoodIdentityJudgement
#[test]
fn identity_error_for_start_proposal2() {
	new_test_ext().execute_with(|| {
		System::set_block_number(0);
		assert_noop!(
			Voting::start_proposal(RuntimeOrigin::signed(2), new_proposal(), 3),
			Error::<Test>::WithoutGoodIdentityJudgement,
		);
	});
}

// Start proposal to low duration - 0
#[test]
fn lowperiod_error_for_start_proposal() {
	new_test_ext().execute_with(|| {
		System::set_block_number(0);
		assert_noop!(
			Voting::start_proposal(RuntimeOrigin::signed(3), new_proposal(), 0),
			Error::<Test>::VotingPeriodLow,
		);
	});
}

// Start proposal successfull flow
#[test]
fn start_proposal_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Voting::start_proposal(RuntimeOrigin::signed(3), new_proposal(), 3));
		System::assert_has_event(RuntimeEvent::Balances(pallet_balances::Event::Reserved {
			who: 3,
			amount: ProposalDeposit::get(),
		}));
		System::assert_has_event(Event::ProposalStarted { account: 3, proposal_index: 0 }.into());
		assert_eq!(System::events().len(), 2);
		//storage check
		assert_eq!(storage_proposal_counter(), 1);
		assert_eq!(storage_deposit(0).unwrap(), (3, ProposalDeposit::get()));
		assert_eq!(
			storage_proposal(0).unwrap(),
			Proposal {
				end: 4,
				proposal: new_proposal(),
				aye: 0,
				nay: 0,
				status: ProposalStatus::ProposalStarted
			}
		);
	});
}

// Start proposal successfull flow many proposals at the same time
#[test]
fn start_many_proposals_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Voting::start_proposal(RuntimeOrigin::signed(3), new_proposal(), 3));
		assert_ok!(Voting::start_proposal(RuntimeOrigin::signed(4), new_proposal(), 4));
		assert_ok!(Voting::start_proposal(RuntimeOrigin::signed(5), new_proposal(), 5));

		System::assert_has_event(RuntimeEvent::Balances(pallet_balances::Event::Reserved {
			who: 3,
			amount: ProposalDeposit::get(),
		}));
		System::assert_has_event(RuntimeEvent::Balances(pallet_balances::Event::Reserved {
			who: 4,
			amount: ProposalDeposit::get(),
		}));
		System::assert_has_event(RuntimeEvent::Balances(pallet_balances::Event::Reserved {
			who: 5,
			amount: ProposalDeposit::get(),
		}));
		System::assert_has_event(Event::ProposalStarted { account: 3, proposal_index: 0 }.into());
		System::assert_has_event(Event::ProposalStarted { account: 4, proposal_index: 1 }.into());
		System::assert_has_event(Event::ProposalStarted { account: 5, proposal_index: 2 }.into());

		//storage check
		assert_eq!(storage_proposal_counter(), 3);
		assert_eq!(storage_deposit(0).unwrap(), (3, ProposalDeposit::get()));
		assert_eq!(
			storage_proposal(0).unwrap(),
			Proposal {
				end: 4,
				proposal: new_proposal(),
				aye: 0,
				nay: 0,
				status: ProposalStatus::ProposalStarted
			}
		);
		assert_eq!(
			storage_proposal(1).unwrap(),
			Proposal {
				end: 5,
				proposal: new_proposal(),
				aye: 0,
				nay: 0,
				status: ProposalStatus::ProposalStarted
			}
		);
		assert_eq!(
			storage_proposal(2).unwrap(),
			Proposal {
				end: 6,
				proposal: new_proposal(),
				aye: 0,
				nay: 0,
				status: ProposalStatus::ProposalStarted
			}
		);
	});
}

// Close proposal tests success and error
// Close proposal wrong proposal index
#[test]
fn index_error_for_close_proposal() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Voting::start_proposal(RuntimeOrigin::signed(3), new_proposal(), 3));
		run_to_block(4);
		assert_noop!(
			Voting::close_proposal(RuntimeOrigin::signed(3), 1),
			Error::<Test>::ProposalIndexInvalid,
		);
	});
}

// Close proposal which is still under voting - error
#[test]
fn under_voting_error_for_close_proposal() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Voting::start_proposal(RuntimeOrigin::signed(3), new_proposal(), 3));
		run_to_block(2);
		assert_noop!(
			Voting::close_proposal(RuntimeOrigin::signed(3), 0),
			Error::<Test>::ProposalUnderVoting,
		);
	});
}

// Close proposal user identity error
#[test]
fn identity_error_for_close_proposal() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Voting::start_proposal(RuntimeOrigin::signed(3), new_proposal(), 3));
		run_to_block(4);
		assert_noop!(
			Voting::close_proposal(RuntimeOrigin::signed(2), 1),
			Error::<Test>::WithoutGoodIdentityJudgement,
		);
	});
}

// Close proposal successfull flow with reject
#[test]
fn close_proposal_rejected_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Voting::start_proposal(RuntimeOrigin::signed(3), new_proposal(), 3));
		assert_ok!(Voting::vote(RuntimeOrigin::signed(3), 0, Vote::Aye(1)));
		assert_ok!(Voting::vote(RuntimeOrigin::signed(4), 0, Vote::Nay(4)));
		run_to_block(5);
		assert_ok!(Voting::close_proposal(RuntimeOrigin::signed(3), 0));
		System::assert_has_event(
			Event::ProposalClosed { proposal_index: 0, status: ProposalStatus::ProposalRejected }
				.into(),
		);
		System::assert_has_event(RuntimeEvent::Balances(pallet_balances::Event::Unreserved {
			who: 3,
			amount: ProposalDeposit::get(),
		}));
		assert_eq!(storage_proposal_counter(), 1);
		assert_eq!(storage_deposit(0), None);
		assert_eq!(storage_proposal(0), None);
	});
}

// Close proposal successfull flow with approve
#[test]
fn close_proposal_approved_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Voting::start_proposal(RuntimeOrigin::signed(3), new_proposal(), 3));
		assert_ok!(Voting::vote(RuntimeOrigin::signed(3), 0, Vote::Aye(4)));

		run_to_block(5);
		assert_ok!(Voting::close_proposal(RuntimeOrigin::signed(3), 0));
		System::assert_has_event(
			Event::ProposalClosed { proposal_index: 0, status: ProposalStatus::ProposalApproved }
				.into(),
		);
		System::assert_has_event(RuntimeEvent::Balances(pallet_balances::Event::Unreserved {
			who: 3,
			amount: ProposalDeposit::get(),
		}));
		assert_eq!(storage_proposal_counter(), 1);
		assert_eq!(storage_deposit(0), None);
		assert_eq!(storage_proposal(0), None);
	});
}

// Close proposal successfull flow with reject
#[test]
fn close_proposal_rejected_works2() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Voting::start_proposal(RuntimeOrigin::signed(3), new_proposal(), 3));
		assert_ok!(Voting::vote(RuntimeOrigin::signed(3), 0, Vote::Aye(4)));
		assert_ok!(Voting::vote(RuntimeOrigin::signed(4), 0, Vote::Nay(1)));
		assert_ok!(Voting::vote(RuntimeOrigin::signed(1), 0, Vote::Nay(2)));
		assert_ok!(Voting::vote(RuntimeOrigin::signed(5), 0, Vote::Nay(1)));

		run_to_block(5);
		assert_ok!(Voting::close_proposal(RuntimeOrigin::signed(3), 0));
		System::assert_has_event(
			Event::ProposalClosed { proposal_index: 0, status: ProposalStatus::ProposalRejected }
				.into(),
		);
		System::assert_has_event(RuntimeEvent::Balances(pallet_balances::Event::Unreserved {
			who: 3,
			amount: ProposalDeposit::get(),
		}));
	});
}

// Close proposal successfull flow with many proposals at the same time
#[test]
fn close_many_proposals_approved_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		let aye = Vote::Aye(2);
		assert_ok!(Voting::start_proposal(RuntimeOrigin::signed(3), new_proposal(), 3));
		assert_ok!(Voting::vote(RuntimeOrigin::signed(3), 0, aye.clone()));
		run_to_block(2);

		assert_ok!(Voting::start_proposal(RuntimeOrigin::signed(3), new_proposal(), 3));
		assert_ok!(Voting::vote(RuntimeOrigin::signed(3), 1, aye));

		run_to_block(5);
		assert_ok!(Voting::close_proposal(RuntimeOrigin::signed(4), 0));
		System::assert_has_event(
			Event::ProposalClosed { proposal_index: 0, status: ProposalStatus::ProposalApproved }
				.into(),
		);
		System::assert_has_event(RuntimeEvent::Balances(pallet_balances::Event::Unreserved {
			who: 3,
			amount: ProposalDeposit::get(),
		}));
		run_to_block(6);

		assert_ok!(Voting::close_proposal(RuntimeOrigin::signed(4), 1));
		System::assert_has_event(
			Event::ProposalClosed { proposal_index: 1, status: ProposalStatus::ProposalApproved }
				.into(),
		);
		System::assert_has_event(RuntimeEvent::Balances(pallet_balances::Event::Unreserved {
			who: 3,
			amount: ProposalDeposit::get(),
		}));

		assert_eq!(storage_proposal_counter(), 2);
		assert_eq!(storage_deposit(0), None);
		assert_eq!(storage_proposal(0), None);
	});
}

// Close bad proposal successfull flow with slash
#[test]
fn close_proposal_reject_slash_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Voting::start_proposal(RuntimeOrigin::signed(3), new_proposal(), 3));
		assert_ok!(Voting::vote(RuntimeOrigin::signed(3), 0, Vote::Nay(4)));

		run_to_block(5);
		assert_ok!(Voting::close_proposal(RuntimeOrigin::signed(3), 0));

		System::assert_has_event(
			Event::ProposalClosed { proposal_index: 0, status: ProposalStatus::ProposalRejected }
				.into(),
		);
		System::assert_has_event(RuntimeEvent::Balances(pallet_balances::Event::Slashed {
			who: 3,
			amount: ProposalDeposit::get(),
		}));
		assert_eq!(storage_proposal_counter(), 1);
		assert_eq!(storage_deposit(0), None);
		assert_eq!(storage_proposal(0), None);
	});
}

// Vote proposal tests success and error
// Vote proposal successfull flow
#[test]
fn vote_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Voting::start_proposal(RuntimeOrigin::signed(3), new_proposal(), 3));
		assert_ok!(Voting::vote(RuntimeOrigin::signed(3), 0, Vote::Aye(4)));
		assert_eq!(
			storage_proposal(0).unwrap(),
			Proposal {
				end: 4,
				proposal: new_proposal(),
				aye: 4,
				nay: 0,
				status: ProposalStatus::ProposalStarted
			}
		);

		run_to_block(5);

		assert_ok!(Voting::close_proposal(RuntimeOrigin::signed(3), 0));
		System::assert_has_event(
			Event::ProposalClosed { proposal_index: 0, status: ProposalStatus::ProposalApproved }
				.into(),
		);
		System::assert_has_event(RuntimeEvent::Balances(pallet_balances::Event::Unreserved {
			who: 3,
			amount: ProposalDeposit::get(),
		}));
		let v: Vec<(ProposalIndex, Vote<Test>)> = vec![(0, Vote::Aye(4))];
		assert_eq!(storage_user_votes(3), ProposalVotes { votes: new_votes_of(v) });
		assert_eq!(storage_proposal_counter(), 1);
		assert_eq!(storage_deposit(0), None);
		assert_eq!(storage_proposal(0), None);
	});
}

// Vote aye proposal successfull flow
#[test]
fn vote_aye_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Voting::start_proposal(RuntimeOrigin::signed(3), new_proposal(), 3));
		let aye = Vote::Aye(4);
		assert_ok!(Voting::vote(RuntimeOrigin::signed(4), 0, aye.clone()));
		System::assert_has_event(
			Event::Voted { account: 4, proposal_index: 0, vote: aye.clone() }.into(),
		);

		let v: Vec<(ProposalIndex, Vote<Test>)> = vec![(0, aye)];
		assert_eq!(storage_user_votes(4), ProposalVotes { votes: new_votes_of(v) });
		assert_eq!(
			storage_proposal(0).unwrap(),
			Proposal {
				end: 4,
				proposal: new_proposal(),
				aye: 4u128,
				nay: 0,
				status: ProposalStatus::ProposalStarted
			}
		);
	});
}

// Vote nay proposal successfull flow
#[test]
fn vote_nay_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Voting::start_proposal(RuntimeOrigin::signed(3), new_proposal(), 3));
		let nay = Vote::Nay(4);
		assert_ok!(Voting::vote(RuntimeOrigin::signed(4), 0, nay.clone()));
		System::assert_has_event(
			Event::Voted { account: 4, proposal_index: 0, vote: nay.clone() }.into(),
		);

		let v: Vec<(ProposalIndex, Vote<Test>)> = vec![(0, nay)];
		assert_eq!(storage_user_votes(4), ProposalVotes { votes: new_votes_of(v) });
		assert_eq!(
			storage_proposal(0).unwrap(),
			Proposal {
				end: 4,
				proposal: new_proposal(),
				aye: 0,
				nay: 4u128,
				status: ProposalStatus::ProposalStarted
			}
		);
	});
}

// Vote 0 nay proposal successfull flow
#[test]
fn vote_nay_0_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Voting::start_proposal(RuntimeOrigin::signed(3), new_proposal(), 3));
		let nay = Vote::Nay(0);
		assert_ok!(Voting::vote(RuntimeOrigin::signed(4), 0, nay.clone()));
		System::assert_has_event(
			Event::Voted { account: 4, proposal_index: 0, vote: nay.clone() }.into(),
		);

		let v: Vec<(ProposalIndex, Vote<Test>)> = vec![(0, nay)];
		assert_eq!(storage_user_votes(4), ProposalVotes { votes: new_votes_of(v) });
		assert_eq!(
			storage_proposal(0).unwrap(),
			Proposal {
				end: 4,
				proposal: new_proposal(),
				aye: 0,
				nay: 0,
				status: ProposalStatus::ProposalStarted
			}
		);
	});
}

// Vote mutliple time as one user successfull flow
#[test]
fn vote_multi_times_same_user_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Voting::start_proposal(RuntimeOrigin::signed(3), new_proposal(), 3));
		let nay = Vote::Nay(4);
		let aye = Vote::Aye(4);
		assert_ok!(Voting::vote(RuntimeOrigin::signed(4), 0, nay.clone()));
		System::assert_has_event(Event::Voted { account: 4, proposal_index: 0, vote: nay }.into());
		assert_ok!(Voting::vote(RuntimeOrigin::signed(4), 0, aye.clone()));
		System::assert_has_event(
			Event::Voted { account: 4, proposal_index: 0, vote: aye.clone() }.into(),
		);
		assert_ok!(Voting::vote(RuntimeOrigin::signed(4), 0, aye.clone()));
		System::assert_has_event(
			Event::Voted { account: 4, proposal_index: 0, vote: aye.clone() }.into(),
		);
		let v: Vec<(ProposalIndex, Vote<Test>)> = vec![(0, aye)];
		assert_eq!(storage_user_votes(4), ProposalVotes { votes: new_votes_of(v) });
		assert_eq!(
			storage_proposal(0).unwrap(),
			Proposal {
				end: 4,
				proposal: new_proposal(),
				aye: 4,
				nay: 0,
				status: ProposalStatus::ProposalStarted
			}
		);
	});
}

// Vote user identity error
#[test]
fn identity_error_for_vote() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Voting::start_proposal(RuntimeOrigin::signed(3), new_proposal(), 3));
		let nay = Vote::Nay(4);
		assert_noop!(
			Voting::vote(RuntimeOrigin::signed(2), 0, nay),
			Error::<Test>::WithoutGoodIdentityJudgement,
		);
	});
}

// Vote max count proposals voted by user error
#[test]
fn max_votes_error_for_vote() {
	new_test_ext().execute_with(|| {
		System::set_block_number(0);

		assert_ok!(Voting::start_proposal(RuntimeOrigin::signed(3), new_proposal(), 3));
		let nay = Vote::Nay(1);

		for i in 0..MaxVotes::get() {
			assert_ok!(Voting::start_proposal(RuntimeOrigin::signed(5), new_proposal(), 3));
			assert_ok!(Voting::vote(RuntimeOrigin::signed(5), i, nay.clone()));
		}
		assert_noop!(Voting::vote(RuntimeOrigin::signed(5), 4, nay), Error::<Test>::VotesMaxCount);
	});
}

// Vote wrong proposals index error
#[test]
fn proposal_index_error_for_vote() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Voting::start_proposal(RuntimeOrigin::signed(3), new_proposal(), 3));
		let nay = Vote::Nay(4);
		assert_noop!(
			Voting::vote(RuntimeOrigin::signed(4), 1, nay),
			Error::<Test>::ProposalIndexInvalid,
		);
	});
}

// Vote max weight low balance error
#[test]
fn max_vote_low_balance_error_for_vote() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Voting::start_proposal(RuntimeOrigin::signed(3), new_proposal(), 3));
		let nay = Vote::Nay(u128::MAX);
		assert_noop!(Voting::vote(RuntimeOrigin::signed(4), 0, nay), Error::<Test>::BalanceToLow,);
	});
}

// Vote for ended proposal error
#[test]
fn proposal_closed_error_for_vote() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Voting::start_proposal(RuntimeOrigin::signed(3), new_proposal(), 3));
		let nay = Vote::Nay(4);
		run_to_block(6);
		assert_noop!(
			Voting::vote(RuntimeOrigin::signed(4), 0, nay),
			Error::<Test>::ProposalClosedForVoting,
		);
	});
}

// Vote from account with to low balance to lock
#[test]
fn balance_low_error_for_vote() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Voting::start_proposal(RuntimeOrigin::signed(3), new_proposal(), 3));
		let nay = Vote::Nay(100);
		run_to_block(6);
		assert_noop!(Voting::vote(RuntimeOrigin::signed(4), 0, nay), Error::<Test>::BalanceToLow,);
	});
}

// Unvote proposal tests success and error
// Unvote successfull flow
#[test]
fn unvote_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(1);
		assert_ok!(Voting::start_proposal(RuntimeOrigin::signed(3), new_proposal(), 3));
		let nay = Vote::Nay(2);
		assert_ok!(Voting::vote(RuntimeOrigin::signed(4), 0, nay.clone()));
		System::assert_has_event(
			Event::Voted { account: 4, proposal_index: 0, vote: nay.clone() }.into(),
		);
		assert_ok!(Voting::unvote(RuntimeOrigin::signed(4), 0));
		System::assert_has_event(
			Event::UnVoted { account: 4, proposal_index: 0, vote: nay }.into(),
		);
		let v: Vec<(ProposalIndex, Vote<Test>)> = vec![];
		assert_eq!(storage_user_votes(4), ProposalVotes { votes: new_votes_of(v) });
		assert_eq!(
			storage_proposal(0).unwrap(),
			Proposal {
				end: 4,
				proposal: new_proposal(),
				aye: 0,
				nay: 0,
				status: ProposalStatus::ProposalStarted
			}
		);
	});
}

// Unvote identity error for user
#[test]
fn identity_error_for_unvote() {
	new_test_ext().execute_with(|| {
		System::set_block_number(0);
		assert_ok!(Voting::start_proposal(RuntimeOrigin::signed(3), new_proposal(), 3));
		assert_noop!(
			Voting::unvote(RuntimeOrigin::signed(2), 0),
			Error::<Test>::WithoutGoodIdentityJudgement,
		);
	});
}

// Unvote proposal using wrong proposal index
#[test]
fn proposal_index_error_for_unvote() {
	new_test_ext().execute_with(|| {
		System::set_block_number(0);
		assert_ok!(Voting::start_proposal(RuntimeOrigin::signed(3), new_proposal(), 3));
		assert_noop!(
			Voting::unvote(RuntimeOrigin::signed(4), 1),
			Error::<Test>::ProposalIndexInvalid,
		);
	});
}

// Unvote proposal which is ended error
#[test]
fn proposal_closed_error_for_unvote() {
	new_test_ext().execute_with(|| {
		System::set_block_number(0);
		assert_ok!(Voting::start_proposal(RuntimeOrigin::signed(3), new_proposal(), 3));
		run_to_block(6);
		assert_noop!(
			Voting::unvote(RuntimeOrigin::signed(4), 0),
			Error::<Test>::ProposalClosedForVoting,
		);
	});
}

// Unvote proposal which is ended error
#[test]
fn proposal_closed_error_for_unvote2() {
	new_test_ext().execute_with(|| {
		System::set_block_number(0);
		assert_ok!(Voting::start_proposal(RuntimeOrigin::signed(3), new_proposal(), 3));
		run_to_block(5);
		assert_ok!(Voting::close_proposal(RuntimeOrigin::signed(3), 0));
		assert_noop!(
			Voting::unvote(RuntimeOrigin::signed(4), 0),
			Error::<Test>::ProposalIndexInvalid,
		);
	});
}

// Unlock account balance tests success and error
// Identity error to unlock balance
#[test]
fn identity_error_for_unlock_account_balance() {
	new_test_ext().execute_with(|| {
		System::set_block_number(0);
		assert_ok!(Voting::start_proposal(RuntimeOrigin::signed(3), new_proposal(), 3));
		assert_noop!(
			Voting::unlock_account_balance(RuntimeOrigin::signed(2), 2, 0),
			Error::<Test>::WithoutGoodIdentityJudgement,
		);
	});
}

// Unlock balance successfull flow
#[test]
fn unlock_account_balance_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(0);
		assert_ok!(Voting::start_proposal(RuntimeOrigin::signed(3), new_proposal(), 3));
		assert_ok!(Voting::vote(RuntimeOrigin::signed(4), 0, Vote::Nay(4)));
		run_to_block(5);
		assert_ok!(Voting::close_proposal(RuntimeOrigin::signed(3), 0));
		assert_eq!(Balances::locks(4), vec![the_lock(16)]);
		assert_ok!(Voting::unlock_account_balance(RuntimeOrigin::signed(4), 4, 0));
		assert_eq!(Balances::locks(4), vec![]);
	});
}

// Unlock balance successfull flow
#[test]
fn unlock_account_balance_works2() {
	new_test_ext().execute_with(|| {
		System::set_block_number(0);
		assert_ok!(Voting::start_proposal(RuntimeOrigin::signed(3), new_proposal(), 3));
		assert_ok!(Voting::vote(RuntimeOrigin::signed(4), 0, Vote::Nay(4)));
		assert_ok!(Voting::vote(RuntimeOrigin::signed(4), 0, Vote::Nay(3)));
		assert_ok!(Voting::unlock_account_balance(RuntimeOrigin::signed(4), 4, 0));
		assert_eq!(Balances::locks(4), vec![the_lock(9)]);

		assert_ok!(Voting::unvote(RuntimeOrigin::signed(4), 0));
		assert_ok!(Voting::unlock_account_balance(RuntimeOrigin::signed(4), 4, 0));

		assert_eq!(Balances::locks(4), vec![]);
		assert_ok!(Voting::vote(RuntimeOrigin::signed(4), 0, Vote::Nay(5)));
		run_to_block(5);
		assert_ok!(Voting::close_proposal(RuntimeOrigin::signed(3), 0));
		assert_eq!(Balances::locks(4), vec![the_lock(25)]);
		assert_ok!(Voting::unlock_account_balance(RuntimeOrigin::signed(4), 4, 0));
		assert_eq!(Balances::locks(4), vec![]);
	});
}

// Unlock balance successfull flow
#[test]
fn unlock_account_balance_works3() {
	new_test_ext().execute_with(|| {
		System::set_block_number(0);
		assert_ok!(Voting::start_proposal(RuntimeOrigin::signed(3), new_proposal(), 3));
		assert_ok!(Voting::start_proposal(RuntimeOrigin::signed(3), new_proposal(), 8));

		assert_ok!(Voting::vote(RuntimeOrigin::signed(4), 1, Vote::Nay(4)));
		assert_ok!(Voting::vote(RuntimeOrigin::signed(4), 0, Vote::Nay(3)));
		assert_eq!(Balances::locks(4), vec![the_lock(16)]);

		assert_ok!(Voting::unvote(RuntimeOrigin::signed(4), 1));
		assert_ok!(Voting::unlock_account_balance(RuntimeOrigin::signed(4), 4, 1));

		assert_eq!(Balances::locks(4), vec![the_lock(9)]);
		assert_ok!(Voting::vote(RuntimeOrigin::signed(4), 1, Vote::Nay(6)));
		run_to_block(4);
		assert_ok!(Voting::close_proposal(RuntimeOrigin::signed(3), 0));
		assert_eq!(Balances::locks(4), vec![the_lock(36)]);
		assert_ok!(Voting::unlock_account_balance(RuntimeOrigin::signed(4), 4, 0));
		assert_eq!(Balances::locks(4), vec![the_lock(36)]);
		run_to_block(4);
		assert_ok!(Voting::unlock_account_balance(RuntimeOrigin::signed(4), 4, 1));
		assert_eq!(Balances::locks(4), vec![the_lock(36)]);
	});
}

// Unlock balance for another account successfull flow
#[test]
fn another_account_unlock_account_balance_works() {
	new_test_ext().execute_with(|| {
		System::set_block_number(0);
		assert_ok!(Voting::start_proposal(RuntimeOrigin::signed(3), new_proposal(), 3));
		assert_ok!(Voting::vote(RuntimeOrigin::signed(4), 0, Vote::Nay(4)));
		run_to_block(5);
		assert_ok!(Voting::close_proposal(RuntimeOrigin::signed(3), 0));
		assert_eq!(Balances::locks(4), vec![the_lock(16)]);
		assert_ok!(Voting::unlock_account_balance(RuntimeOrigin::signed(3), 4, 0));
		assert_eq!(Balances::locks(4), vec![]);
	});
}
