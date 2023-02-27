# Quadratic Voting

## Assignment definition
Assignment is defined in ASSIGNMENT.md

## Overview

The Voting pallets allows users for quadratic voting for proposals.
To join the voting, user needs to get identity. Identity management is implemented
using identity pallet.

Terms:
proposal - the subject on which user vote. It can be anything - it is just an hash for pallet,
vote - user defines the vote weight which wants to use for vote, from which is calculated an account balance lock,
'proposal_index' - each proposal gets index which is incremented with each new proposal, starting from 0,
balance lock - balance on user account is locked in order to use it in voting. It protects voting system from
transfering tokens to different accounts just to do the voting. Account balance lock is square of vote weight.
reserved balance - when starting a proposal, user account balance is reserved to protect system against bad proposals(spam).
When proposal is identified as a spam user account will be slashed. An proposal is considered as a bad one,
when X% of total voting weights is against it. X is configured as SlashThreshold.

## Voting process:
#### Proposal start
First the proposal needs to be started. It can be done by any user with identity. The proposal is related
to a hash of proposal and a voting period in blocks. Proposal starts immediately after call to start_proposal.
There can be many ongoing proposals at the same time.

#### Voting
From this point users can start to vote. User can vote and unvote. When unvoted, the previous vote is deleted. In this case the
account balance lock is still present until user call unlock_account_balance. Unlocking account balance is a heavy operation,
which requires iteration over a vector, so it is not called automatically. When user voted and votes again on the same proposal
the previous vote is replaced with the new one. User account balance lock is of the size of the biggest account balance
used in voting. Users can vote as long as the proposal is ongoing. Voting is not allowed after the voting period has ended.
Users can vote on multiple proposals at the same time.

#### Proposal close
Proposal needs to be closed using close_proposal function. It can be done only if voting period has ended
proposal_close function calculates a voting result and emits the event with the result. 
After this user can unlock his account balance. Proposal is accepted when over 50% votes are aye, in the other case is rejected.
Storage data for proposal is removed. To clean storage for votes per user, call to unlock_account_balance is needed.

#### State changes

For simplification is considered users u1 and u2 and one proposal at time (implementation allows for many proposal at the same time)
```
Action                                        State
                      | proposal, proposal owner reserved balance, votes count | user locked balance |
Initial state         | --      , --                             , --          | --                  |                      |
Start proposal(u1) -> | 1       , u1                             , --          | --                  |
Vote(u2,2)         -> | 1       , u1                             , 2           | 4                   |
UnVote(u2)         -> | 1       , u1                             , --          | 4                   |
Vote(u2,2)         -> | 1       , u1                             , 2           | 4                   |
Close proposal     -> | --      , --                             , --          | 4                   |
Unlock balance(u2) -> | --      , --                             , --          | --                  |
```
Call "start_proposal" adds new proposal for which is associated proposal index, as a handle to operate on proposal.
On proposal owner account is reserved balance, which can be slashed when proposal is bad. When proposal is successfull
event is emited

When proposal is ongoing user can vote and unvote. To each user vote is attached its weight. Lock balance for vote is a square
of it weight. On user account is locked balance, which is the max balance value of all votes user did on ongoing proposals.
Unvote is removing the previous vote for the proposal. Unvote does not change the locked account balance.
Both calls vote and unvote emits events, with votes type - aye, nay and vote weight.

Proposal can be closed after voting ends. It calculates voting result and emits event with it.
Proposal owner reserved balance is unreserved or slashed depending on proposal quality.
An proposal is considered as a bad one, when X% of total voting weights is against it. X is configured as SlashThreshold.

Unlock account balance call can be made anytime, but make sense after proposal has ended or after user did unvote
and wants to reset lock or unlock account balance.


## Considerations

#### Proposal handle
proposal handle - proposal identifier in calls and events. It could be used hash of proposal but it takes much space 
when we want to keep relations in structures in memory or in storage to proposals. With proposal index as u32
it could be used also compact wrapper to compact it when kept in encoded form.
Using internal u32 we can use faster hash algorithm in maps, does not need to be secure one.
Max of u32 is big enough to keep all proposals in the future. MaxProposal is configurable.

#### integer_sqrt
integer_sqrt - there were two option, use this function to calculate vote weight or user should specify the weight directly
in vote. Use of this function can introduce rounding issues, but from other side user directly knows how much of balance will be locked.
In implementation user needs to specify the vote weight, from which is calculated account balance lock.

#### votes 0 weight
Votes with 0 weight - such votes do not impact voting, but are allowed by system

#### Starting the same proposal
A malicious user can start proposals which has been already voted.  
When the voting history is not kept due to storage space, we can deregister identity for such user
and additionally the account balance can be slashed when the proposal is cosidered as a bad one.

#### Starting bad proposals
Account balance slash is used in order to protect the system against it

