pragma solidity ^0.5.0;

// A sample contract demontrating signatures, mappings, structures and intercontract communtications

contract IMyContract {
	function getCredit() public;
}

contract IMyContractCallback {
	function getCreditCallback(uint64 balance) public;
}

contract MyContract is IMyContract {

	struct ContractInfo {
		uint64	allowed;
	}

	mapping(address => ContractInfo) m_allowed;
	uint256 m_ownerPubkey;
	
	// TVM helpers
	function tvm_sender_pubkey() pure private returns (uint256) {}
	function tvm_logstr(bytes32) pure private {}
	modifier tvm_signed() { _; }
	
	// External messages
	
	constructor() public tvm_signed {
		tvm_logstr("constructor");
		uint256 pubkey = tvm_sender_pubkey();
		//require(pubkey != 0);
		m_ownerPubkey = pubkey;
	}
	
	// a kind of modifier
	function ensureOwner() view private {
		require(tvm_sender_pubkey() == m_ownerPubkey);
	}
	
	function setAllowance(address anotherContract, uint64 amount) public tvm_signed {
		ensureOwner();
		m_allowed[anotherContract].allowed = amount;
	}
	
	// Internal messages

	function getCredit() public {
		IMyContractCallback(msg.sender).getCreditCallback(m_allowed[msg.sender].allowed);
		return;
	}
	
}
