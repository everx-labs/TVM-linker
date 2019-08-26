pragma solidity ^0.5.0;

import "contract06.sol";

// A sample contract demontrating signatures, mappings, structures and intercontract communtications

contract MyContract is IMyContract {

	// struct for info about credit
	struct ContractInfo {
		uint64	allowed;
	}

	// persistent variable storing a credit infomation for some addresses
	mapping(address => ContractInfo) m_allowed;
	
	// TVM helpers
	function tvm_logstr(bytes32) pure private {}
	
	// External messages
	
	// set the credit limit for the address
	function setAllowance(address anotherContract, uint64 amount) public {
		m_allowed[anotherContract].allowed = amount;
	}
	
	// Internal messages

	function getCredit() public {
		// cast calleer to IMyContractCallback and call method getCreditCallback
		// with value getted from persistent variable
		IMyContractCallback(msg.sender).getCreditCallback(m_allowed[msg.sender].allowed);
		return;
	}
	
}
