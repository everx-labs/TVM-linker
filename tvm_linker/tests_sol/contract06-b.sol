pragma solidity ^0.5.0;

contract IMyContract {
	function getCredit() public;
}

contract IMyContractCallback {
	function getCreditCallback(uint64 balance) public;
}

contract RemoteContract is IMyContractCallback {

	uint64 m_credit;
	
	// external methods
	
	function getMyCredit(IMyContract bank) public {
		// call method of remote contract
		bank.getCredit();
		return;
	}
	
	// interface IMyContractCallback
	
	function getCreditCallback(uint64 balance) public {
		// save balance of credit (received from another contract) in persistent variable
		m_credit = balance;
	}
	
}
