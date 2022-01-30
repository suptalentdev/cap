# Internet Computer History OIS Spec

This document goes into the interface description of the Internet Computer History OIS
without implementation. To keep the name short we refer to this service as ICHS in this
document.

The ICHS is a service which provides scalable transaction history that can be used by any
number of tokens or NFTs to store their event log and issue a transaction id to the end
user. It also provides a unified view to the history for every token that integrates this
service for client-facing wallets and network scan UIs.

The primary goal of this project is filling the gap for a native unified ETH-like history
on the Internet Computer.

In this document we go thought the overall architecture of such a service and provide the
schema for the canisters that the system has.

This interface should work regardless of the number of canister the ICHS uses. We define
two groups of canisters. `Readable` and `Writable`. Every canister on the ICHS implements
one or both of these interfaces. For example the entry/router canister implements both
*Readable* and *Writable*

The goal of defining this interfaces is to have implementation agnostic layer of
interaction with this open internet service and future proofing the OIS.

## Readable

The *Readable* interface describes a common schema for performing read-only queries on
the ICHS. We refer to a canister that implements this interface as a Readable Canister.
This interface does not define any operation on the canister that can mutate the state
of the canister.

Not every readable canister is capable of returning the response for the entire data,
and they should be called in the right context. Starting from the entry canister should
always result in valid responses.

## Writable

The *Writable* interface describes an interface for canisters that can mutate the state
of the ICHS. For example inserting a new event to the history.

## Certified Trees

Each `Readable` canister should have one merkle-tree whose root hash is stored as the
certified data of the entire canister.

This tree is defined using the following structure:

```
HashTree
  = Empty
  | Fork HashTree HashTree
  | Labeled Label HashTree
  | Leaf blob
  | Pruned Hash
Label = Blob
Hash = Blob
Signature = Blob
```

The schema above describes a tree which has binary blobs as key and values of the tree.
Any number inserted as either label or leaves on tree should be stored in Big Endian.

We define the following operations on the `HashTree`:

1. `reconstruct` should provide the root hash of the tree.
```
reconstruct(Empty)       = H(domain_sep("ic-hashtree-empty"))
reconstruct(Fork t1 t2)  = H(domain_sep("ic-hashtree-fork") · reconstruct(t1) · reconstruct(t2))
reconstruct(Labeled l t) = H(domain_sep("ic-hashtree-labeled") · l · reconstruct(t))
reconstruct(Leaf v)      = H(domain_sep("ic-hashtree-leaf") · v)
reconstruct(Pruned h)    = h

domain_sep(s) = byte(|s|) · s
```

2. `flatten` should eliminate the pruned nodes and return the most inner tree, of course
the new tree will have a different root hash, so the tree obtained should already be
certified.
```
flatten(Fork Pruned t) = flatten(t)
flatten(Fork t Pruned) = flatten(t)
flatten(t) = t
```

The alias `tree<K, V>` in this doc refers to a tree with labels of type `K` and leaves of
type `V`.

Type alias `leaf<T>` refers to a tree node that has the given data type.

The `flatten_fork<T1, T2>` refers to a tree that once flattened, it'll have two sub-trees
with the given types, the `T1` is the left/first subtree and `T2` is the right/second
tree.

For example `flatten_fork<tree<u32, TransactionHash>, leaf<u64>>` means that the most
inner subtree of the root tree should be a fork that has two nodes, one of which is
another tree that maps `u32` values to a `TransactionHash` and the second node is a
`u64` constant number.

## Transactions

The transaction type determines the shape of each transaction that can be inserted or
queried from the ICHS service.

A transaction is described by the following candid interface:

```
type Event = variant {
    // The original caller to the `insert` method.
    token  : principal;
    // The time the transaction was inserted to ICHS in ms.
    time   : u64;
    // Should be the original caller who invoked the call on the token canister.
    caller : principal;
    // The amount touched in the event.
    amount : u64;
    // The fee that was captured by the token.
    // --- QUESTION: What is the unit for the fee?
    fee    : u64;
    // A memo for this transaction.
    memo   : u32;
    // Details of the event.
    kind   : EventKind;
};

type IndefiniteEvent = variant {
    caller : principal;
    amount : u64;
    fee    : u64;
    memo   : u32;
    kind   : EventKind;
};

type EventKind = variant {
    Transfer : record {
        from : principal;
        to   : principal;
    };
    Mint     : record {
        to   : principal;
    };
    Burn     : record {
        from: principal;
        to: opt principal;
    };
    Custom   : record {
        name : text;
        spenders: vec principal;
        receivers: vec principal;
    };
};
```

Now we describe how you can obtain a hash from a `Event`, the most important rule is that
every field in the `Event` should be part of the process of generating the hash.

```
hash_event(Event token time caller amount fee memo Transfer from to) =
    H(domain_sep("transfer")
    . byte(time) . byte(amount) . byte(fee) . byte(memo)
    . token . caller . from . to)

hash_event(Event token time caller amount fee memo Mint to) =
    H(domain_sep("mint")
    . byte(time) . byte(amount) . byte(fee) . byte(memo)
    . token . caller . to)
    
hash_event(Event token time caller amount fee memo Burn from null) =
    H(domain_sep("burn")
    . byte(time) . byte(amount) . byte(fee) . byte(memo)
    . token . caller . from)
    
hash_event(Event token time caller amount fee memo Burn from to) =
    H(domain_sep("burn")
    . byte(time) . byte(amount) . byte(fee) . byte(memo)
    . token . caller . from . to)
    
hash_event(Event token time caller amount fee memo Custom name spenders receivers) =
    H(domain_sep("burn")
    . name
    . byte(time) . byte(amount) . byte(fee) . byte(memo)
    . token . caller . concat(spenders) . concat(receivers))
```

## Readable Canister

```
type ReadableCanisterId = principal;

type Witness = record {
    certificate: blob;
    // CBOR serialized HashTree
    tree: blob;
};

type EventHash = blob;

type TransactionId = nat64;

type WithIdArg = record {
    id: TransactionId;
    witness: bool;
};

type GetTransactionResponse = variant {
    // Witness type: tree<TransactionId, ReadableCanisterId>
    Delegate(principal, opt Witness),
    // Witness type: flatten_fork<tree<nat32, EventHash>, leaf<TransactionId>>
    Found(Event, opt Witness)
};

// [nat8; 34] = byte(principal) . byte(nat32)
// 30 bytes for principal, the first byte is the len.
// 4 bytes for page number.
type PageKey = blob;

// Hash a page of events. See the section below for Page Hash.
type PageHash = blob;

type WithPageArg = record {
    principal: principal;
    page: nat32;
    witness: bool;
};

type GetTransactionsResponse = struct {
    data: vec Event;
    // Witness type: tree<PageKey, PageHash>
    witness: opt Witness;
};

type WithWitnessArg = record {
    witness: bool;
};

type GetIndexCanistersResponse = record {
    canisters: vec ReadableCanisterId;
    // Witness type: leaf(CanistersListHash)
    // CanistersListHash is computed like events page.
    witness: opt Witness;
}

type GetBucketResponse = record {
    canister: ReadableCanisterId;
    // Witness type: tree<TransactionId, ReadableCanisterId>
    witness: opt Witness;
};

service readable : {
    // Return the list of canisters that can be used for routing the requests.
    get_index_canisters : (WithWitnessArg) -> (GetIndexCanistersResponse) query;

    // Return a bucket that can be used to query for the given transaction id.
    get_bucket_for : (WithIdArg) -> (GetBucketResponse) query;

    // Return the given transaction.
    get_transaction : (WithIdArg) -> (GetTransactionResponse) query;

    // Return all of the transactions associated with the given user.
    get_user_transactions : (WithPageArg) -> (GetTransactionsResponse) query;

    // Return all of the transactions associated with the given token.
    get_token_transactions : (WithPageArg) -> (GetTransactionsResponse) query;
};
```

### Page Hash

```
hash_page(vec) = [0; 32]
hash_page(vec ..events event) = H(hash_page(events) . hash_event(event))
```

## Writable Canister

```
type WritableCanisterId = principal;

type TransactionId = nat64;

service writable : {
    // Return the canisters that can be used to write data to.
    get_writer_canisters : () -> (vec WritableCanisterId);

    // Insert the given transaction to the ICHS and issue a transaction id.
    insert : (IndefiniteEvent) -> (TransactionId);

    // The time on the canister. The time can be used to check if this WritableCanister
    // is on the same subnet as the caller.
    time : () -> (nat64) query;
};
```
