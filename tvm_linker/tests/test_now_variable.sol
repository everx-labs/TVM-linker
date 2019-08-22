pragma solidity ^0.5.0;

contract TestNow {

    function main() public payable {
        require((now > 1566494745) && (now < 1566494745 + 60*60*24*356));
    }
}